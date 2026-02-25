use crate::tui::state::{CachedScan, FolderEntry, SpaceLensState};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

const EXCLUDED_PATHS: &[&str] = &["/System/Volumes", "/Volumes", "/dev", "/.vol"];

type Job = (PathBuf, String, bool, Sender<FolderEntry>);

static ACTIVE_THREADS_4: AtomicUsize = AtomicUsize::new(0);
static ACTIVE_THREADS_8: AtomicUsize = AtomicUsize::new(0);
static ACTIVE_THREADS_16: AtomicUsize = AtomicUsize::new(0);

pub fn get_active_threads(pool_size: usize) -> usize {
    match pool_size {
        1..=4 => ACTIVE_THREADS_4.load(Ordering::SeqCst),
        5..=8 => ACTIVE_THREADS_8.load(Ordering::SeqCst),
        _ => ACTIVE_THREADS_16.load(Ordering::SeqCst),
    }
}

struct ThreadPool {
    job_sender: Sender<Job>,
}

impl ThreadPool {
    fn new(size: usize, active_counter: &'static AtomicUsize) -> Self {
        let (job_tx, job_rx): (Sender<Job>, Receiver<Job>) = channel();
        let job_rx = Arc::new(Mutex::new(job_rx));

        for _ in 0..size {
            let rx = Arc::clone(&job_rx);
            let counter = active_counter;
            thread::spawn(move || loop {
                let job = {
                    let rx = rx.lock().unwrap();
                    rx.recv()
                };
                match job {
                    Ok((path, name, is_dir, result_tx)) => {
                        counter.fetch_add(1, Ordering::SeqCst);
                        if is_dir {
                            let mut current_size: u64 = 0;
                            for e in walkdir::WalkDir::new(&path)
                                .same_file_system(true)
                                .into_iter()
                                .filter_entry(|e| {
                                    for excluded in EXCLUDED_PATHS {
                                        if e.path().starts_with(excluded) {
                                            return false;
                                        }
                                    }
                                    !e.path_is_symlink()
                                })
                                .filter_map(|e| e.ok())
                            {
                                if let Ok(metadata) = e.metadata() {
                                    if metadata.is_file() {
                                        current_size += metadata.len();
                                        let _ = result_tx.send(FolderEntry {
                                            name: name.clone(),
                                            path: path.clone(),
                                            size: current_size,
                                            is_dir,
                                            scanning: true,
                                        });
                                    }
                                }
                            }
                            let _ = result_tx.send(FolderEntry {
                                name,
                                path: path.clone(),
                                size: current_size,
                                is_dir,
                                scanning: false,
                            });
                        } else if let Ok(metadata) = path.metadata() {
                            let _ = result_tx.send(FolderEntry {
                                name,
                                path,
                                size: metadata.len(),
                                is_dir,
                                scanning: false,
                            });
                        }
                        counter.fetch_sub(1, Ordering::SeqCst);
                    }
                    Err(_) => break,
                }
            });
        }

        ThreadPool { job_sender: job_tx }
    }

    fn submit(&self, job: Job) {
        let _ = self.job_sender.send(job);
    }
}

static POOL_4: OnceLock<ThreadPool> = OnceLock::new();
static POOL_8: OnceLock<ThreadPool> = OnceLock::new();
static POOL_16: OnceLock<ThreadPool> = OnceLock::new();

fn get_thread_pool(size: usize) -> &'static ThreadPool {
    match size {
        1..=4 => POOL_4.get_or_init(|| ThreadPool::new(4, &ACTIVE_THREADS_4)),
        5..=8 => POOL_8.get_or_init(|| ThreadPool::new(8, &ACTIVE_THREADS_8)),
        _ => POOL_16.get_or_init(|| ThreadPool::new(16, &ACTIVE_THREADS_16)),
    }
}

pub fn start_space_scan(state: &mut SpaceLensState) {
    let path = state.current_path.clone();
    let thread_count = state.thread_count;

    // 캐시 확인
    let should_rescan = if let Some(cached) = state.cache.get(&path).cloned() {
        state.entries = cached.entries;
        state.total_size = cached.total_size;
        // 로딩 중이었으면 스캔 재개 필요
        cached.was_loading
    } else {
        // 캐시 없으면 새 스캔
        state.entries.clear();
        state.total_size = 0;
        true
    };

    if !should_rescan {
        state.loading = false;
        return;
    }

    if !state.parallel_scan {
        // 단일 모드에서도 현재 경로가 아닌 이전 스캔은 유지
        // (복귀 시 스캔 재개를 위해)
        let current = state.current_path.clone();
        state.pending_scans.retain(|path, _| *path == current);
    }

    // 이미 스캔 중이면 기존 결과 사용
    if state.pending_scans.contains_key(&path) {
        state.loading = true;
        return;
    }

    state.loading = true;

    let (_tx, rx) = channel();
    state.pending_scans.insert(path.clone(), rx);

    let _parallel = state.parallel_scan;
    let has_existing_entries = !state.entries.is_empty();

    let (tx, rx) = channel();
    state.pending_scans.insert(path.clone(), rx);

    let parallel = state.parallel_scan;

    // 메인 스레드에서 read_dir 수행 (빠름)
    let entries: Vec<_> = if let Ok(read_dir) = std::fs::read_dir(&path) {
        read_dir
            .flatten()
            .filter(|e| {
                let entry_path = e.path();

                // 제외 경로 체크
                for excluded in EXCLUDED_PATHS {
                    if entry_path.starts_with(excluded) {
                        return false;
                    }
                }

                // 심볼릭 링크 제외
                if entry_path.is_symlink() {
                    return false;
                }

                // 다른 파일시스템(마운트 포인트) 제외
                if entry_path.is_dir() {
                    if let (Ok(entry_metadata), Ok(root_metadata)) = (e.metadata(), path.metadata())
                    {
                        if entry_metadata.dev() != root_metadata.dev() {
                            return false;
                        }
                    }
                }
                true
            })
            .collect()
    } else {
        return;
    };

    // Phase 1: 모든 항목 전송 (size=0, scanning=true)
    // 이미 있는 항목이면 스킵 (이미 스캔 완료되거나 진행 중)
    for entry in &entries {
        let entry_path = entry.path();

        // 이미 entries에 있고 스캔 완료된 상태면 스킵
        if has_existing_entries {
            if let Some(existing) = state.entries.iter().find(|e| e.path == entry_path) {
                if !existing.scanning {
                    continue; // 이미 완료됨
                }
            }
        }

        let name = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        let is_dir = entry_path.is_dir();
        let _ = tx.send(FolderEntry {
            name,
            path: entry_path.clone(),
            size: 0,
            is_dir,
            scanning: true,
        });
    }

    // Phase 2: 백그라운드에서 크기 계산
    if parallel {
        let pool = get_thread_pool(thread_count);
        'outer: for entry in entries {
            let tx = tx.clone();
            let entry_path = entry.path();
            let name = entry_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();
            let is_dir = entry_path.is_dir();

            // 이미 entries에 있고 스캔 완료된 항목은 skip
            if is_dir && has_existing_entries {
                if let Some(existing) = state.entries.iter().find(|e| e.path == entry_path) {
                    if !existing.scanning {
                        continue;
                    }
                }
            }

            // 캐시에 이미 완료된 스캔이 있으면 재사용
            if is_dir {
                if let Some(cached) = state.cache.get(&entry_path) {
                    if !cached.was_loading {
                        let _ = tx.send(FolderEntry {
                            name,
                            path: entry_path.clone(),
                            size: cached.total_size,
                            is_dir,
                            scanning: false,
                        });
                        continue 'outer;
                    }
                }
            }

            pool.submit((entry_path, name, is_dir, tx));
        }
    } else {
        // 단일 모드: 캐시와 entries 복사해서 스레드로 전달
        let cache_clone = state.cache.clone();
        let existing_entries = state.entries.clone();
        let has_existing = has_existing_entries;

        thread::spawn(move || {
            'outer: for entry in entries {
                let entry_path = entry.path();
                let name = entry_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string();
                let is_dir = entry_path.is_dir();

                // 이미 entries에 있고 스캔 완료된 항목은 skip
                if is_dir && has_existing {
                    if let Some(existing) = existing_entries.iter().find(|e| e.path == entry_path) {
                        if !existing.scanning {
                            continue;
                        }
                    }
                }

                // 캐시에 이미 완료된 스캔이 있으면 재사용
                if is_dir {
                    if let Some(cached) = cache_clone.get(&entry_path) {
                        if !cached.was_loading {
                            let _ = tx.send(FolderEntry {
                                name,
                                path: entry_path.clone(),
                                size: cached.total_size,
                                is_dir,
                                scanning: false,
                            });
                            continue 'outer;
                        }
                    }
                }

                if is_dir {
                    let mut current_size: u64 = 0;
                    for e in walkdir::WalkDir::new(&entry_path)
                        .same_file_system(true)
                        .into_iter()
                        .filter_entry(|e| {
                            for excluded in EXCLUDED_PATHS {
                                if e.path().starts_with(excluded) {
                                    return false;
                                }
                            }
                            !e.path_is_symlink()
                        })
                        .filter_map(|e| e.ok())
                    {
                        if let Ok(metadata) = e.metadata() {
                            if metadata.is_file() {
                                current_size += metadata.len();
                                let _ = tx.send(FolderEntry {
                                    name: name.clone(),
                                    path: entry_path.clone(),
                                    size: current_size,
                                    is_dir,
                                    scanning: true,
                                });
                            }
                        }
                    }
                    // 완료 신호
                    let _ = tx.send(FolderEntry {
                        name,
                        path: entry_path.clone(),
                        size: current_size,
                        is_dir,
                        scanning: false,
                    });
                } else if let Ok(metadata) = entry_path.metadata() {
                    let _ = tx.send(FolderEntry {
                        name,
                        path: entry_path,
                        size: metadata.len(),
                        is_dir,
                        scanning: false,
                    });
                }
            }
        });
    }
}

pub fn poll_space_sizes(state: &mut SpaceLensState) {
    let current_path = state.current_path.clone();
    let mut completed_paths: Vec<PathBuf> = Vec::new();

    for (path, rx) in &mut state.pending_scans {
        loop {
            match rx.try_recv() {
                Ok(entry) => {
                    if *path == current_path {
                        if let Some(existing) =
                            state.entries.iter_mut().find(|e| e.path == entry.path)
                        {
                            // size=0은 아직 스캔 중인 상태, 기존 크기가 있으면 덮어쓰지 않음
                            if entry.size > 0 || existing.size == 0 {
                                state.total_size = state.total_size.saturating_sub(existing.size);
                                existing.size = entry.size;
                                state.total_size += entry.size;
                            }
                            // scanning 상태 업데이트
                            existing.scanning = entry.scanning;
                        } else {
                            state.total_size += entry.size;
                            state.entries.push(entry);
                        }
                        state.entries.sort_by(|a, b| b.size.cmp(&a.size));
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    completed_paths.push(path.clone());
                    if *path == current_path {
                        state.loading = false;
                        state.cache.insert(
                            current_path.clone(),
                            CachedScan {
                                entries: state.entries.clone(),
                                total_size: state.total_size,
                                was_loading: false,
                            },
                        );
                    }
                    break;
                }
            }
        }
    }

    for path in completed_paths {
        state.pending_scans.remove(&path);
    }
}
