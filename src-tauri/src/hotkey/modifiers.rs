//! Shared modifier tracking for Windows keyboard hooks.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri_plugin_global_shortcut::Modifiers;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VIRTUAL_KEY, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU,
    VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
};

#[derive(Debug, Default)]
pub struct TrackedModifiers {
    ctrl: AtomicBool,
    alt: AtomicBool,
    shift: AtomicBool,
    super_key: AtomicBool,
}

impl TrackedModifiers {
    pub const fn new() -> Self {
        Self {
            ctrl: AtomicBool::new(false),
            alt: AtomicBool::new(false),
            shift: AtomicBool::new(false),
            super_key: AtomicBool::new(false),
        }
    }

    pub fn reset(&self) {
        self.ctrl.store(false, Ordering::SeqCst);
        self.alt.store(false, Ordering::SeqCst);
        self.shift.store(false, Ordering::SeqCst);
        self.super_key.store(false, Ordering::SeqCst);
    }

    pub fn set_vk(&self, vk: VIRTUAL_KEY, pressed: bool) {
        match vk {
            VK_CONTROL | VK_LCONTROL | VK_RCONTROL => {
                self.ctrl.store(pressed, Ordering::SeqCst);
            }
            VK_MENU | VK_LMENU | VK_RMENU => {
                self.alt.store(pressed, Ordering::SeqCst);
            }
            VK_SHIFT | VK_LSHIFT | VK_RSHIFT => {
                self.shift.store(pressed, Ordering::SeqCst);
            }
            VK_LWIN | VK_RWIN => {
                self.super_key.store(pressed, Ordering::SeqCst);
            }
            _ => {}
        }
    }

    pub fn labels(&self) -> Vec<&'static str> {
        let mut parts = self.labels_tracked();
        if !parts.contains(&"Ctrl") && modifier_pressed(VK_CONTROL) {
            parts.push("Ctrl");
        }
        if !parts.contains(&"Alt") && modifier_pressed(VK_MENU) {
            parts.push("Alt");
        }
        if !parts.contains(&"Shift") && modifier_pressed(VK_SHIFT) {
            parts.push("Shift");
        }
        if !parts.contains(&"Super") && (modifier_pressed(VK_LWIN) || modifier_pressed(VK_RWIN)) {
            parts.push("Super");
        }
        parts
    }

    /// Hook-tracked modifiers only (ignores live keyboard state).
    pub fn labels_tracked(&self) -> Vec<&'static str> {
        let mut parts = Vec::new();
        if self.ctrl.load(Ordering::SeqCst) {
            parts.push("Ctrl");
        }
        if self.alt.load(Ordering::SeqCst) {
            parts.push("Alt");
        }
        if self.shift.load(Ordering::SeqCst) {
            parts.push("Shift");
        }
        if self.super_key.load(Ordering::SeqCst) {
            parts.push("Super");
        }
        parts
    }

    pub fn labels_with_vk(&self, vk: VIRTUAL_KEY) -> Vec<&'static str> {
        let mut parts = self.labels();
        if is_modifier_vk(vk) {
            let label = vk_modifier_label(vk);
            if !parts.contains(&label) {
                parts.push(label);
            }
            parts.sort_by_key(|label| modifier_sort_key(label));
            parts.dedup();
        }
        parts
    }

    pub fn required_modifiers_satisfied(&self, required: Modifiers) -> bool {
        (!required.contains(Modifiers::CONTROL)
            || self.ctrl.load(Ordering::SeqCst)
            || modifier_pressed(VK_CONTROL))
            && (!required.contains(Modifiers::ALT)
                || self.alt.load(Ordering::SeqCst)
                || modifier_pressed(VK_MENU))
            && (!required.contains(Modifiers::SHIFT)
                || self.shift.load(Ordering::SeqCst)
                || modifier_pressed(VK_SHIFT))
            && (!required.contains(Modifiers::SUPER)
                || self.super_key.load(Ordering::SeqCst)
                || modifier_pressed(VK_LWIN)
                || modifier_pressed(VK_RWIN))
    }
}

pub fn is_modifier_vk(vk: VIRTUAL_KEY) -> bool {
    matches!(
        vk,
        VK_CONTROL
            | VK_LCONTROL
            | VK_RCONTROL
            | VK_MENU
            | VK_LMENU
            | VK_RMENU
            | VK_SHIFT
            | VK_LSHIFT
            | VK_RSHIFT
            | VK_LWIN
            | VK_RWIN
    )
}

pub fn modifier_pressed(vk: VIRTUAL_KEY) -> bool {
    unsafe { (GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000) != 0 }
}

pub fn vk_modifier_label(vk: VIRTUAL_KEY) -> &'static str {
    match vk {
        VK_CONTROL | VK_LCONTROL | VK_RCONTROL => "Ctrl",
        VK_MENU | VK_LMENU | VK_RMENU => "Alt",
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT => "Shift",
        VK_LWIN | VK_RWIN => "Super",
        _ => "Ctrl",
    }
}

pub fn modifier_sort_key(label: &str) -> u8 {
    match label {
        "Ctrl" => 0,
        "Alt" => 1,
        "Shift" => 2,
        "Super" => 3,
        _ => 4,
    }
}

pub fn compose_modifier_only(labels: &[&str]) -> Option<String> {
    if labels.len() < 2 {
        return None;
    }
    let mut parts = labels.to_vec();
    parts.sort_by_key(|label| modifier_sort_key(label));
    parts.dedup();
    if parts.len() < 2 {
        return None;
    }
    Some(parts.join("+"))
}
