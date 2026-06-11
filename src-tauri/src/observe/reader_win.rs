//! Windows UI Automation text reader for the focused control.

use uiautomation::controls::ControlType;
use uiautomation::core::UIElement;
use uiautomation::patterns::{UITextPattern, UIValuePattern};
use uiautomation::UIAutomation;

thread_local! {
    static UIA: std::cell::RefCell<Option<UIAutomation>> = const { std::cell::RefCell::new(None) };
}

fn focused_in_own_process(element: &UIElement) -> bool {
    let own_pid = std::process::id() as i32;
    let mut current = element.clone();
    for _ in 0..8 {
        if current.get_process_id().ok() == Some(own_pid) {
            return true;
        }
        if let Ok(parent) = current.get_cached_parent() {
            current = parent;
        } else {
            break;
        }
    }
    false
}

pub fn read_focused_text() -> Option<String> {
    UIA.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = UIAutomation::new().ok();
        }
        read_focused_text_with(slot.as_ref()?)
    })
}

fn read_focused_text_with(automation: &UIAutomation) -> Option<String> {
    let element = automation.get_focused_element().ok()?;
    if focused_in_own_process(&element) {
        return None;
    }

    if let Ok(value_pattern) = element.get_pattern::<UIValuePattern>() {
        if let Ok(value) = value_pattern.get_value() {
            if !value.is_empty() {
                return Some(value);
            }
        }
    }

    if let Ok(text_pattern) = element.get_pattern::<UITextPattern>() {
        if let Ok(range) = text_pattern.get_document_range() {
            if let Ok(text) = range.get_text(-1) {
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
    }

    // Walk up to a parent edit/document control when focus is on a leaf (e.g. Notepad).
    let mut current = element;
    for _ in 0..4 {
        if let Ok(parent) = current.get_cached_parent() {
            current = parent;
            if let Ok(control_type) = current.get_control_type() {
                if matches!(
                    control_type,
                    ControlType::Edit | ControlType::Document | ControlType::Text
                ) {
                    if let Ok(value_pattern) = current.get_pattern::<UIValuePattern>() {
                        if let Ok(value) = value_pattern.get_value() {
                            if !value.is_empty() {
                                return Some(value);
                            }
                        }
                    }
                    if let Ok(text_pattern) = current.get_pattern::<UITextPattern>() {
                        if let Ok(range) = text_pattern.get_document_range() {
                            if let Ok(text) = range.get_text(-1) {
                                if !text.is_empty() {
                                    return Some(text);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            break;
        }
    }

    None
}
