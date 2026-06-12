//! Low-level keyboard hook for reliable hotkey capture on Windows (WebView2 misses many combos).

use std::sync::Mutex;

use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VIRTUAL_KEY, VK_0, VK_1, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8, VK_9, VK_A, VK_B, VK_BACK,
    VK_C, VK_D, VK_E, VK_ESCAPE, VK_F, VK_F1, VK_F10, VK_F11, VK_F12, VK_F2, VK_F3, VK_F4, VK_F5,
    VK_F6, VK_F7, VK_F8, VK_F9, VK_G, VK_H, VK_I, VK_J, VK_K, VK_L, VK_M, VK_N, VK_O, VK_P, VK_Q,
    VK_R, VK_RETURN, VK_S, VK_SPACE, VK_T, VK_TAB, VK_U, VK_V, VK_W, VK_X, VK_Y, VK_Z,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, LLKHF_UP,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::modifiers::{compose_modifier_only, is_modifier_vk, TrackedModifiers};
use super::parse_hotkey_setting;

const PREVIOUS_KEY_DOWN: u32 = 1 << 30;

static CAPTURE: Mutex<Option<HotkeyCaptureSession>> = Mutex::new(None);
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

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
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
            let labels = MODIFIERS.labels_with_vk(vk);
            if let Some(hotkey) = compose_modifier_only(&labels) {
                if parse_hotkey_setting(&hotkey).is_ok() {
                    if let Some(app) = CAPTURE
                        .lock()
                        .ok()
                        .and_then(|guard| guard.as_ref().map(|session| session.app.clone()))
                    {
                        emit_on_main_thread(&app, "hotkey-captured", Some(hotkey));
                    }
                }
            }
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
        if parse_hotkey_setting(&hotkey).is_ok() {
            emit_on_main_thread(&app, "hotkey-captured", Some(hotkey));
        } else {
            emit_on_main_thread(&app, "hotkey-capture-invalid", None);
        }
    }

    CallNextHookEx(hook, code, wparam, lparam)
}

fn compose_hotkey_from_vk(vk: VIRTUAL_KEY) -> Option<String> {
    let labels = MODIFIERS.labels();
    if labels.is_empty() {
        return None;
    }

    let key = vk_to_key_label(vk)?;
    let mut parts = labels;
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
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_LCONTROL, VK_LMENU};

    #[test]
    fn maps_space_vk_to_label() {
        assert_eq!(vk_to_key_label(VK_SPACE), Some("Space"));
    }

    #[test]
    fn rejects_modifier_vk_as_key() {
        assert!(is_modifier_vk(VK_LCONTROL));
    }

    #[test]
    fn tracked_modifiers_compose_ctrl_a() {
        MODIFIERS.reset();
        MODIFIERS.set_vk(VK_LCONTROL, true);
        let mut parts = MODIFIERS.labels_tracked();
        parts.push("A");
        assert_eq!(Some(parts.join("+")), Some("Ctrl+A".into()));
        MODIFIERS.reset();
    }

    #[test]
    fn tracked_modifiers_compose_ctrl_alt_on_release() {
        MODIFIERS.reset();
        MODIFIERS.set_vk(VK_LCONTROL, true);
        MODIFIERS.set_vk(VK_LMENU, true);
        let labels = MODIFIERS.labels_tracked();
        assert_eq!(compose_modifier_only(&labels), Some("Ctrl+Alt".into()));
        MODIFIERS.reset();
    }
}
