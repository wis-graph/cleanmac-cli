#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    CategorySelect,
    Review,
    ConfirmClean,
    ResultDisplay,
    Help,
    AppList,
    LoadingRelatedFiles,
    UninstallReview,
    UninstallResult,
    SpaceLens,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum SortMode {
    #[default]
    SizeDesc,
    SizeAsc,
    NameAsc,
    NameDesc,
}

impl SortMode {
    pub fn next(self) -> Self {
        match self {
            SortMode::SizeDesc => SortMode::SizeAsc,
            SortMode::SizeAsc => SortMode::NameAsc,
            SortMode::NameAsc => SortMode::NameDesc,
            SortMode::NameDesc => SortMode::SizeDesc,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SortMode::SizeDesc => "Size ↓",
            SortMode::SizeAsc => "Size ↑",
            SortMode::NameAsc => "Name A-Z",
            SortMode::NameDesc => "Name Z-A",
        }
    }
}
