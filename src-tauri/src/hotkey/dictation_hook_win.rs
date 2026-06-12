//! Permanent low-level hook for modifier-only dictation hotkeys (Ctrl+Alt, Ctrl+Super, …).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Modifiers, ShortcutState};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, LLKHF_UP,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::modifiers::{is_modifier_vk, TrackedModifiers};
use crate::dispatch_dictation_hotkey;

const PREVIOUS_KEY_DOWN: u32 = 1 << 30;

static HOOK: Mutex<Option<DictationHookSession>> = Mutex::new(None);
static MODIFIERS: TrackedModifiers = TrackedModifiers::new();
static CHORD_ACTIVE: AtomicBool = AtomicBool::new(false);

struct DictationHookSession {
    app: AppHandle,
    required: Modifiers,
    hook: isize,
}

pub fn start_modifier_dictation_hook(app: &AppHandle, required: Modifiers) -> Result<(), String> {
    let mut guard = HOOK
        .lock()
        .map_err(|_| "dictation hook lock poisoned".to_string())?;
    if let Some(session) = guard.as_ref() {
        if session.required == required {
            return Ok(());
        }
        stop_locked(&mut guard)?;
    }

    MODIFIERS.reset();
    CHORD_ACTIVE.store(false, Ordering::SeqCst);

    let app = app.clone();
    let hook = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0)
            .map_err(|e| format!("failed to install dictation keyboard hook: {e}"))?
    };

    *guard = Some(DictationHookSession {
        app,
        required,
        hook: hook.0 as isize,
    });
    Ok(())
}

pub fn stop_modifier_dictation_hook() -> Result<(), String> {
    let mut guard = HOOK
        .lock()
        .map_err(|_| "dictation hook lock poisoned".to_string())?;
    stop_locked(&mut guard)
}

fn stop_locked(guard: &mut Option<DictationHookSession>) -> Result<(), String> {
    let Some(session) = guard.take() else {
        return Ok(());
    };

    CHORD_ACTIVE.store(false, Ordering::SeqCst);
    MODIFIERS.reset();
    unsafe {
        UnhookWindowsHookEx(HHOOK(session.hook as *mut core::ffi::c_void))
            .map_err(|e| format!("failed to remove dictation keyboard hook: {e}"))?;
    }
    Ok(())
}

fn current_hook() -> Option<HHOOK> {
    HOOK.lock().ok().and_then(|guard| {
        guard
            .as_ref()
            .map(|session| HHOOK(session.hook as *mut core::ffi::c_void))
    })
}

fn required_modifiers() -> Option<Modifiers> {
    HOOK.lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|s| s.required))
}

fn hook_app() -> Option<AppHandle> {
    HOOK.lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|session| session.app.clone()))
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
    if is_key_down && (kb.flags.contains(LLKHF_UP) || kb.flags.0 & PREVIOUS_KEY_DOWN != 0) {
        return CallNextHookEx(hook, code, wparam, lparam);
    }

    let vk = VIRTUAL_KEY(kb.vkCode as u16);
    let Some(required) = required_modifiers() else {
        return CallNextHookEx(hook, code, wparam, lparam);
    };

    if is_modifier_vk(vk) {
        MODIFIERS.set_vk(vk, is_key_down);
    } else if is_key_down {
        return CallNextHookEx(hook, code, wparam, lparam);
    }

    let satisfied = MODIFIERS.required_modifiers_satisfied(required);
    let was_active = CHORD_ACTIVE.load(Ordering::SeqCst);

    if satisfied && !was_active {
        CHORD_ACTIVE.store(true, Ordering::SeqCst);
        if let Some(app) = hook_app() {
            dispatch_dictation_hotkey(&app, ShortcutState::Pressed);
        }
    } else if was_active && !satisfied {
        CHORD_ACTIVE.store(false, Ordering::SeqCst);
        if let Some(app) = hook_app() {
            dispatch_dictation_hotkey(&app, ShortcutState::Released);
        }
    }

    CallNextHookEx(hook, code, wparam, lparam)
}
