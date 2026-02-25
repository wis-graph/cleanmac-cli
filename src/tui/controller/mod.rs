pub mod app_list;
pub mod category_select;
pub mod common;
pub mod review;
pub mod space_lens;
pub mod uninstall;

pub use app_list::handle_app_list_key;
pub use category_select::handle_category_select_key;
pub use common::{handle_confirm_key, handle_help_key, handle_result_key};
pub use review::handle_review_key;
pub use space_lens::handle_space_lens_key;
pub use uninstall::{handle_uninstall_result_key, handle_uninstall_review_key};
