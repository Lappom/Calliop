//! Low-level keyboard hook for reliable hotkey capture on Windows (WebView2 misses many combos).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VIRTUAL_KEY, VK_0, VK_1, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8, VK_9,
    VK_A, VK_B, VK_BACK, VK_C, VK_CONTROL, VK_D, VK_E, VK_ESCAPE, VK_F, VK_F1, VK_F10, VK_F11,
    VK_F12, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_G, VK_H, VK_I, VK_J, VK_K,
    VK_L, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_M, VK_MENU, VK_N, VK_O, VK_P, VK_Q, VK_R,
    VK_RETURN, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_S, VK_SHIFT, VK_SPACE, VK_T, VK_TAB,
    VK_U, VK_V, VK_W, VK_X, VK_Y, VK_Z,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, LLKHF_UP,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::parse_shortcut;

const PREVIOUS_KEY_DOWN: u32 = 1 << 30;

static CAPTURE: Mutex<Option<HotkeyCaptureSession>> = Mutex::new(None);

struct TrackedModifiers {
    ctrl: AtomicBool,
    alt: AtomicBool,
    shift: AtomicBool,
    super_key: AtomicBool,
}

impl TrackedModifiers {
    const fn new() -> Self {
        Self {
            ctrl: AtomicBool::new(false),
            alt: AtomicBool::new(false),
            shift: AtomicBool::new(false),
            super_key: AtomicBool::new(false),
        }
    }

    fn reset(&self) {
        self.ctrl.store(false, Ordering::SeqCst);
        self.alt.store(false, Ordering::SeqCst);
        self.shift.store(false, Ordering::SeqCst);
        self.super_key.store(false, Ordering::SeqCst);
    }

    fn set_vk(&self, vk: VIRTUAL_KEY, pressed: bool) {
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

    fn any_pressed(&self) -> bool {
        self.ctrl.load(Ordering::SeqCst)
            || self.alt.load(Ordering::SeqCst)
            || self.shift.load(Ordering::SeqCst)
            || self.super_key.load(Ordering::SeqCst)
            || modifier_pressed(VK_CONTROL)
            || modifier_pressed(VK_MENU)
            || modifier_pressed(VK_SHIFT)
            || modifier_pressed(VK_LWIN)
            || modifier_pressed(VK_RWIN)
    }

    fn labels(&self) -> Vec<&'static str> {
        let mut parts = Vec::new();
        if self.ctrl.load(Ordering::SeqCst) || modifier_pressed(VK_CONTROL) {
            parts.push("Ctrl");
        }
        if self.alt.load(Ordering::SeqCst) || modifier_pressed(VK_MENU) {
            parts.push("Alt");
        }
        if self.shift.load(Ordering::SeqCst) || modifier_pressed(VK_SHIFT) {
            parts.push("Shift");
        }
        if self.super_key.load(Ordering::SeqCst)
            || modifier_pressed(VK_LWIN)
            || modifier_pressed(VK_RWIN)
        {
            parts.push("Super");
        }
        parts
    }
}

static MODIFIERS: TrackedModifiers = TrackedModifiers::new();

struct HotkeyCaptureSession {
    app: AppHandle,
    hook: isize,
}

pub fn start(app: &AppHandle) -> Result<bool, String> {
    let mut guard = CAPTURE
        .lock()
        .map_err(|_| "hotkey capture lock poisoned".to_string())?;
    if guard.is_some() {
        return Ok(true);
    }

    MODIFIERS.reset();
    let app = app.clone();
    let hook = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0)
            .map_err(|e| format!("failed to install keyboard hook: {e}"))?
    };

    *guard = Some(HotkeyCaptureSession {
        app,
        hook: hook.0 as isize,
    });
    Ok(true)
}

pub fn stop() -> Result<(), String> {
    let mut guard = CAPTURE
        .lock()
        .map_err(|_| "hotkey capture lock poisoned".to_string())?;
    let Some(session) = guard.take() else {
        return Ok(());
    };

    MODIFIERS.reset();
    unsafe {
        UnhookWindowsHookEx(HHOOK(session.hook as *mut core::ffi::c_void))
            .map_err(|e| format!("failed to remove keyboard hook: {e}"))?;
    }
    Ok(())
}

fn current_hook() -> Option<HHOOK> {
    CAPTURE.lock().ok().and_then(|guard| {
        guard
            .as_ref()
            .map(|session| HHOOK(session.hook as *mut core::ffi::c_void))
    })
}

fn emit_on_main_thread(app: &AppHandle, event: &'static str, payload: Option<String>) {
    let app_for_thread = app.clone();
    let app_for_emit = app.clone();
    let _ = app_for_thread.run_on_main_thread(move || match payload {
        Some(value) => {
            let _ = app_for_emit.emit(event, value);
        }
        None => {
            let _ = app_for_emit.emit(event, ());
        }
    });
}

unsafe extern "system" fn keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let hook = match current_hook() {
        Some(hook) => hook,
        None => return CallNextHookEx(HHOOK::default(), code, wparam, lparam),
    };

    if code < 0 {
        return CallNextHookEx(hook, code, wparam, lparam);
    }

    let msg = wparam.0 as u32;
    let is_key_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let is_key_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
    if !is_key_down && !is_key_up {
        return CallNextHookEx(hook, code, wparam, lparam);
    }

    let kb = *(lparam.0 as *const KBDLLHOOKSTRUCT);
    let vk = VIRTUAL_KEY(kb.vkCode as u16);

    if is_key_up {
        if is_modifier_vk(vk) {
            MODIFIERS.set_vk(vk, false);
        }
        return CallNextHookEx(hook, code, wparam, lparam);
    }

    if kb.flags.contains(LLKHF_UP) || kb.flags.0 & PREVIOUS_KEY_DOWN != 0 {
        return CallNextHookEx(hook, code, wparam, lparam);
    }

    if is_modifier_vk(vk) {
        MODIFIERS.set_vk(vk, true);
        return CallNextHookEx(hook, code, wparam, lparam);
    }

    let Some(app) = CAPTURE
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|session| session.app.clone()))
    else {
        return CallNextHookEx(hook, code, wparam, lparam);
    };

    if vk == VK_ESCAPE {
        emit_on_main_thread(&app, "hotkey-capture-cancelled", None);
        return CallNextHookEx(hook, code, wparam, lparam);
    }

    if let Some(hotkey) = compose_hotkey_from_vk(vk) {
        if parse_shortcut(&hotkey).is_ok() {
            emit_on_main_thread(&app, "hotkey-captured", Some(hotkey));
        } else {
            emit_on_main_thread(&app, "hotkey-capture-invalid", None);
        }
    }

    CallNextHookEx(hook, code, wparam, lparam)
}

fn is_modifier_vk(vk: VIRTUAL_KEY) -> bool {
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

fn modifier_pressed(vk: VIRTUAL_KEY) -> bool {
    unsafe { (GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000) != 0 }
}

fn compose_hotkey_from_vk(vk: VIRTUAL_KEY) -> Option<String> {
    if !MODIFIERS.any_pressed() {
        return None;
    }

    let key = vk_to_key_label(vk)?;
    let mut parts = MODIFIERS.labels();
    if parts.is_empty() {
        return None;
    }
    parts.push(key);
    Some(parts.join("+"))
}

fn vk_to_key_label(vk: VIRTUAL_KEY) -> Option<&'static str> {
    Some(match vk {
        VK_SPACE => "Space",
        VK_RETURN => "Enter",
        VK_TAB => "Tab",
        VK_BACK => "Backspace",
        VK_ESCAPE => "Escape",
        VK_F1 => "F1",
        VK_F2 => "F2",
        VK_F3 => "F3",
        VK_F4 => "F4",
        VK_F5 => "F5",
        VK_F6 => "F6",
        VK_F7 => "F7",
        VK_F8 => "F8",
        VK_F9 => "F9",
        VK_F10 => "F10",
        VK_F11 => "F11",
        VK_F12 => "F12",
        VK_0 => "0",
        VK_1 => "1",
        VK_2 => "2",
        VK_3 => "3",
        VK_4 => "4",
        VK_5 => "5",
        VK_6 => "6",
        VK_7 => "7",
        VK_8 => "8",
        VK_9 => "9",
        VK_A => "A",
        VK_B => "B",
        VK_C => "C",
        VK_D => "D",
        VK_E => "E",
        VK_F => "F",
        VK_G => "G",
        VK_H => "H",
        VK_I => "I",
        VK_J => "J",
        VK_K => "K",
        VK_L => "L",
        VK_M => "M",
        VK_N => "N",
        VK_O => "O",
        VK_P => "P",
        VK_Q => "Q",
        VK_R => "R",
        VK_S => "S",
        VK_T => "T",
        VK_U => "U",
        VK_V => "V",
        VK_W => "W",
        VK_X => "X",
        VK_Y => "Y",
        VK_Z => "Z",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_space_vk_to_label() {
        assert_eq!(vk_to_key_label(VK_SPACE), Some("Space"));
    }

    #[test]
    fn rejects_modifier_vk_as_key() {
        assert!(is_modifier_vk(VK_LCONTROL));
        assert!(is_modifier_vk(VK_LWIN));
    }

    #[test]
    fn tracked_modifiers_compose_ctrl_a() {
        MODIFIERS.reset();
        MODIFIERS.set_vk(VK_LCONTROL, true);
        assert_eq!(compose_hotkey_from_vk(VK_A), Some("Ctrl+A".into()));
        MODIFIERS.reset();
    }
}
