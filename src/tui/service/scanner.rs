use crate::config::Config;
use crate::plugin::{
    registry::{CategoryScanResult, ScanReport},
    ScanConfig, Scanner, ScannerCategory,
};
use crate::scanner::{
    BrowserCacheScanner, CacheScanner, DevJunkScanner, DuplicatesScanner, LargeOldFilesScanner,
    LogScanner, MailAttachmentsScanner, MaintenanceScanner, MusicJunkScanner, PhotoJunkScanner,
    PrivacyScanner, StartupItemsScanner, TrashScanner,
};
use crate::tui::state::{AppMode, ScanMessage, ScanProgress};
use ratatui::widgets::ListState;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

const DEFAULT_POOL_SIZE: usize = 4;

type ScannerJob = (
    Box<dyn Scanner>,
    ScannerCategory,
    Sender<ScanMessage>,
    ScanConfig,
    Arc<AtomicUsize>,
);

struct ScannerPool {
    job_sender: Sender<ScannerJob>,
}

impl ScannerPool {
    fn new(size: usize) -> Self {
        let (job_tx, job_rx): (Sender<ScannerJob>, Receiver<ScannerJob>) = channel();
        let job_rx = Arc::new(Mutex::new(job_rx));

        for _ in 0..size {
            let rx = Arc::clone(&job_rx);
            thread::spawn(move || loop {
                let job = {
                    let rx = rx.lock().unwrap();
                    rx.try_recv()
                };
                match job {
                    Ok((scanner, category, tx, scan_config, completed)) => {
                        let scanner_name = scanner.name().to_string();
                        let scanner_id = scanner.id().to_string();

                        let _ = tx.send(ScanMessage::ScannerStart {
                            name: scanner_name.clone(),
                        });

                        let _ = scanner.scan(&scan_config);

                        let _ = tx.send(ScanMessage::ScannerDone {
                            scanner_id,
                            name: scanner_name.clone(),
                            category,
                        });

                        completed.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(TryRecvError::Empty) => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(TryRecvError::Disconnected) => break,
                }
            });
        }

        ScannerPool { job_sender: job_tx }
    }

    fn submit(&self, job: ScannerJob) {
        let _ = self.job_sender.send(job);
    }
}

static POOL: OnceLock<ScannerPool> = OnceLock::new();

fn get_scanner_pool() -> &'static ScannerPool {
    POOL.get_or_init(|| ScannerPool::new(DEFAULT_POOL_SIZE))
}

pub struct ScanStartParams<'a> {
    pub config: &'a Config,
    pub enabled_scanner_ids: Vec<String>,
    pub report: &'a mut Option<ScanReport>,
    pub scan_progress: &'a mut ScanProgress,
    pub scan_receiver: &'a mut Option<Receiver<ScanMessage>>,
    pub mode: &'a mut AppMode,
}

pub fn start_scan(params: &mut ScanStartParams) {
    let enabled_ids = params.enabled_scanner_ids.clone();

    if enabled_ids.is_empty() {
        return;
    }

    let (tx, rx) = channel();

    let progress_tx = tx.clone();
    let item_tx = tx.clone();
    let scan_config = ScanConfig {
        min_size: params.config.scan.min_size_bytes,
        max_depth: params.config.scan.max_depth,
        excluded_paths: params
            .config
            .scan
            .excluded_paths
            .iter()
            .map(|s| PathBuf::from(s))
            .collect(),
        progress_callback: Some(std::sync::Arc::new(move |path: &str| {
            let _ = progress_tx.send(ScanMessage::ScanningPath {
                path: path.to_string(),
            });
        })),
        item_callback: Some(std::sync::Arc::new(move |item| {
            let scanner_id = item.metadata.get("scanner_id").cloned().unwrap_or_default();
            let _ = item_tx.send(ScanMessage::ItemFound { scanner_id, item });
        })),
    };

    if let Some(ref mut report) = params.report {
        let removed_size: u64 = report
            .categories
            .iter()
            .filter(|c| enabled_ids.contains(&c.scanner_id))
            .map(|c| c.total_size())
            .sum();
        let removed_items: usize = report
            .categories
            .iter()
            .filter(|c| enabled_ids.contains(&c.scanner_id))
            .map(|c| c.items.len())
            .sum();

        report
            .categories
            .retain(|c| !enabled_ids.contains(&c.scanner_id));
        report.total_size = report.total_size.saturating_sub(removed_size);
        report.total_items = report.total_items.saturating_sub(removed_items);
    } else {
        *params.report = Some(ScanReport {
            categories: Vec::new(),
            total_size: 0,
            total_items: 0,
            duration: Duration::from_secs(0),
        });
    }

    *params.scan_progress = ScanProgress {
        current_scanner: "Initializing...".to_string(),
        current_path: None,
        scanners_done: 0,
        total_scanners: enabled_ids.len(),
        active_scanners: 0,
    };
    *params.scan_receiver = Some(rx);
    *params.mode = AppMode::Review;

    let all_scanners: Vec<(String, Box<dyn Scanner>, ScannerCategory)> = vec![
        (
            "system_caches".into(),
            Box::new(CacheScanner::new()) as Box<dyn Scanner>,
            ScannerCategory::System,
        ),
        (
            "system_logs".into(),
            Box::new(LogScanner::new()) as Box<dyn Scanner>,
            ScannerCategory::System,
        ),
        (
            "trash".into(),
            Box::new(TrashScanner::new()) as Box<dyn Scanner>,
            ScannerCategory::Trash,
        ),
        (
            "browser_caches".into(),
            Box::new(BrowserCacheScanner::new()) as Box<dyn Scanner>,
            ScannerCategory::Browser,
        ),
        (
            "dev_junk".into(),
            Box::new(DevJunkScanner::new()) as Box<dyn Scanner>,
            ScannerCategory::Development,
        ),
        (
            "large_old_files".into(),
            Box::new(LargeOldFilesScanner::new()) as Box<dyn Scanner>,
            ScannerCategory::System,
        ),
        (
            "mail_attachments".into(),
            Box::new(MailAttachmentsScanner::new()) as Box<dyn Scanner>,
            ScannerCategory::System,
        ),
        (
            "photo_junk".into(),
            Box::new(PhotoJunkScanner::new()) as Box<dyn Scanner>,
            ScannerCategory::System,
        ),
        (
            "music_junk".into(),
            Box::new(MusicJunkScanner::new()) as Box<dyn Scanner>,
            ScannerCategory::System,
        ),
        (
            "duplicates".into(),
            Box::new(DuplicatesScanner::new()) as Box<dyn Scanner>,
            ScannerCategory::System,
        ),
        (
            "privacy".into(),
            Box::new(PrivacyScanner::new()) as Box<dyn Scanner>,
            ScannerCategory::Browser,
        ),
        (
            "maintenance".into(),
            Box::new(MaintenanceScanner::new()) as Box<dyn Scanner>,
            ScannerCategory::System,
        ),
        (
            "startup_items".into(),
            Box::new(StartupItemsScanner::new()) as Box<dyn Scanner>,
            ScannerCategory::System,
        ),
    ];

    let scanners: Vec<_> = all_scanners
        .into_iter()
        .filter(|(id, _, _)| enabled_ids.contains(id))
        .collect();

    let total = scanners.len();
    let pool = get_scanner_pool();
    let completed_count = Arc::new(AtomicUsize::new(0));

    thread::spawn(move || {
        for (_id, scanner, category) in scanners.into_iter() {
            pool.submit((
                scanner,
                category,
                tx.clone(),
                scan_config.clone(),
                Arc::clone(&completed_count),
            ));
        }

        loop {
            thread::sleep(Duration::from_millis(100));
            if completed_count.load(Ordering::SeqCst) >= total {
                break;
            }
        }

        let _ = tx.send(ScanMessage::ScanComplete);
    });
}

pub struct PollContext<'a> {
    pub scan_receiver: &'a mut Option<Receiver<ScanMessage>>,
    pub report: &'a mut Option<ScanReport>,
    pub scan_progress: &'a mut ScanProgress,
    pub list_state: &'a mut ListState,
}

pub fn poll_scan_messages(ctx: &mut PollContext) {
    let rx_opt = ctx.scan_receiver.take();
    if let Some(ref rx) = rx_opt {
        let mut complete = false;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                ScanMessage::ScannerStart { name, .. } => {
                    ctx.scan_progress.current_scanner = name;
                    ctx.scan_progress.active_scanners += 1;
                }
                ScanMessage::ScanningPath { path } => {
                    ctx.scan_progress.current_path = Some(path);
                }
                ScanMessage::ItemFound { scanner_id, item } => {
                    if let Some(ref mut report) = ctx.report {
                        report.total_size += item.size;
                        report.total_items += 1;

                        if let Some(cat) = report
                            .categories
                            .iter_mut()
                            .find(|c| c.scanner_id == scanner_id)
                        {
                            cat.items.push(item);
                        } else {
                            let new_cat = CategoryScanResult {
                                scanner_id: scanner_id.clone(),
                                name: scanner_id.clone(),
                                category: ScannerCategory::System,
                                items: vec![item],
                            };
                            report.categories.push(new_cat);
                            if report.categories.len() == 1 {
                                ctx.list_state.select(Some(0));
                            }
                        }
                    }
                }
                ScanMessage::ScannerDone {
                    scanner_id,
                    name,
                    category,
                } => {
                    if let Some(ref mut report) = ctx.report {
                        if let Some(cat) = report
                            .categories
                            .iter_mut()
                            .find(|c| c.scanner_id == scanner_id)
                        {
                            cat.name = name;
                            cat.category = category;
                        }
                    }
                    ctx.scan_progress.scanners_done += 1;
                    ctx.scan_progress.active_scanners =
                        ctx.scan_progress.active_scanners.saturating_sub(1);
                    ctx.scan_progress.current_path = None;
                }
                ScanMessage::ScanComplete { .. } => {
                    complete = true;
                }
            }
        }
        if !complete {
            *ctx.scan_receiver = rx_opt;
        }
    }
}
