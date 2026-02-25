use crate::plugin::ScanResult;
use std::collections::HashSet;

pub fn toggle_selection(selected_items: &mut HashSet<String>, focused_item: Option<&ScanResult>) {
    if let Some(item) = focused_item {
        let id = item.id.clone();
        if selected_items.contains(&id) {
            selected_items.remove(&id);
        } else {
            selected_items.insert(id);
        }
    }
}

pub fn select_all_in_category(selected_items: &mut HashSet<String>, items: &[ScanResult]) {
    for item in items {
        selected_items.insert(item.id.clone());
    }
}

pub fn deselect_all(selected_items: &mut HashSet<String>) {
    selected_items.clear();
}
