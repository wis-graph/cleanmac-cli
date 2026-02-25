use crate::plugin::registry::ScanReport;
use ratatui::widgets::ListState;

pub fn navigate_up(list_state: &mut ListState) {
    if let Some(current) = list_state.selected() {
        if current > 0 {
            list_state.select(Some(current - 1));
        }
    }
}

pub fn navigate_down(list_state: &mut ListState, max_items: usize) {
    let max = max_items.saturating_sub(1);
    if let Some(current) = list_state.selected() {
        if current < max {
            list_state.select(Some(current + 1));
        }
    }
}

pub fn navigate_category_prev(selected_category: &mut usize, list_state: &mut ListState) {
    if *selected_category > 0 {
        *selected_category -= 1;
        list_state.select(Some(0));
    }
}

pub fn navigate_category_next(
    selected_category: &mut usize,
    list_state: &mut ListState,
    report: Option<&ScanReport>,
) {
    if let Some(report) = report {
        if *selected_category < report.categories.len().saturating_sub(1) {
            *selected_category += 1;
            list_state.select(Some(0));
        }
    }
}
