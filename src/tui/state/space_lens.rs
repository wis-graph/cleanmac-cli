use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone)]
pub struct FolderEntry {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub scanning: bool,
}

#[derive(Clone)]
pub struct CachedScan {
    pub entries: Vec<FolderEntry>,
    pub total_size: u64,
    pub was_loading: bool,
}

pub struct SpaceLensState {
    pub current_path: PathBuf,
    pub entries: Vec<FolderEntry>,
    pub total_size: u64,
    pub loading: bool,
    pub cache: HashMap<PathBuf, CachedScan>,
    pub pending_scans: HashMap<PathBuf, Receiver<FolderEntry>>,
    pub parallel_scan: bool,
    pub thread_count: usize,
}

impl Default for SpaceLensState {
    fn default() -> Self {
        Self {
            current_path: PathBuf::from("/"),
            entries: Vec::new(),
            total_size: 0,
            loading: false,
            cache: HashMap::new(),
            pending_scans: HashMap::new(),
            parallel_scan: true,
            thread_count: 4,
        }
    }
}
