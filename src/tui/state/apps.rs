use crate::uninstaller::{AppBundle, RelatedFile};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone, Default)]
pub struct UninstallResultDisplay {
    pub app_deleted: bool,
    pub related_deleted: usize,
    pub total_freed: u64,
    pub errors: Vec<String>,
}

pub struct AppsModeState {
    pub apps: Vec<AppBundle>,
    pub app_sizes: HashMap<usize, u64>,
    pub selected_app_idx: Option<usize>,
    pub selected_related: HashSet<usize>,
    pub uninstall_result: Option<UninstallResultDisplay>,
    pub cached_related_files: Vec<RelatedFile>,
    pub size_receiver: Option<Receiver<(usize, u64)>>,
}

impl Default for AppsModeState {
    fn default() -> Self {
        Self {
            apps: Vec::new(),
            app_sizes: HashMap::new(),
            selected_app_idx: None,
            selected_related: HashSet::new(),
            uninstall_result: None,
            cached_related_files: Vec::new(),
            size_receiver: None,
        }
    }
}
