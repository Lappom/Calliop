//! Post-injection correction observation via Windows UI Automation (Phase 3b+).

mod region;
mod watcher;

#[cfg(not(windows))]
mod reader_stub;
#[cfg(windows)]
mod reader_win;

#[cfg(not(windows))]
use reader_stub as reader;
#[cfg(windows)]
use reader_win as reader;

pub use region::{build_anchors, extract_region, InjectionAnchors};
pub use watcher::{spawn_correction_watcher, CorrectionHandler};

pub fn read_focused_text() -> Option<String> {
    reader::read_focused_text()
}

pub fn supports_correction_watcher() -> bool {
    cfg!(windows)
}

pub fn module_name() -> &'static str {
    "observe"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_module_name() {
        assert_eq!(module_name(), "observe");
    }
}
