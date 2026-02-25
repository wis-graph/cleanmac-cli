use crate::plugin::registry::ScanReport;
use crate::tui::state::SortMode;

pub fn apply_sort(report: &mut ScanReport, sort_mode: SortMode) {
    for category in &mut report.categories {
        match sort_mode {
            SortMode::SizeDesc => {
                category.items.sort_by(|a, b| b.size.cmp(&a.size));
            }
            SortMode::SizeAsc => {
                category.items.sort_by(|a, b| a.size.cmp(&b.size));
            }
            SortMode::NameAsc => {
                category.items.sort_by(|a, b| {
                    a.path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_lowercase()
                        .cmp(
                            &b.path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_lowercase(),
                        )
                });
            }
            SortMode::NameDesc => {
                category.items.sort_by(|a, b| {
                    b.path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_lowercase()
                        .cmp(
                            &a.path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_lowercase(),
                        )
                });
            }
        }
    }
}
