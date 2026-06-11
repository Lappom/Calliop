//! Windows foreground window detection via Win32 APIs.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND, MAX_PATH};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
};

use super::ActiveWindow;

static LAST_EXTERNAL_FOREGROUND: Mutex<Option<ActiveWindow>> = Mutex::new(None);
static FOREGROUND_POLL_STARTED: OnceLock<()> = OnceLock::new();

const POLL_INTERVAL: Duration = Duration::from_millis(400);

pub fn ensure_foreground_poll_started() {
    FOREGROUND_POLL_STARTED.get_or_init(|| {
        thread::spawn(|| loop {
            if let Some(window) = read_foreground_window() {
                if !is_calliop_window(&window) {
                    if let Ok(mut cache) = LAST_EXTERNAL_FOREGROUND.lock() {
                        *cache = Some(window);
                    }
                }
            }
            thread::sleep(POLL_INTERVAL);
        });
    });
}

pub fn get_active_window() -> Option<ActiveWindow> {
    ensure_foreground_poll_started();

    if let Some(current) = read_foreground_window() {
        if !is_calliop_window(&current) {
            if let Ok(mut cache) = LAST_EXTERNAL_FOREGROUND.lock() {
                *cache = Some(current.clone());
            }
            return Some(current);
        }
    }

    LAST_EXTERNAL_FOREGROUND
        .lock()
        .ok()
        .and_then(|cache| cache.clone())
}

fn read_foreground_window() -> Option<ActiveWindow> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return None;
    }

    let title = read_window_title(hwnd);
    let process_id = window_process_id(hwnd);
    let (exe_name, exe_path) = read_process_info(hwnd).unwrap_or((String::new(), None));

    Some(ActiveWindow {
        title,
        exe_name,
        exe_path,
        process_id,
    })
}

fn read_window_title(hwnd: HWND) -> String {
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length <= 0 {
        return String::new();
    }

    let mut buffer = vec![0_u16; (length as usize) + 1];
    let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    if copied <= 0 {
        return String::new();
    }
    buffer.truncate(copied as usize);
    String::from_utf16_lossy(&buffer)
}

fn window_process_id(hwnd: HWND) -> Option<u32> {
    let mut process_id = 0_u32;
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
            hwnd,
            Some(&mut process_id),
        );
    }
    (process_id != 0).then_some(process_id)
}

fn read_process_info(hwnd: HWND) -> Option<(String, Option<String>)> {
    let process_id = window_process_id(hwnd)?;

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()? };

    let result = (|| {
        let mut buffer = vec![0_u16; MAX_PATH as usize];
        let mut size = buffer.len() as u32;
        unsafe {
            QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut size,
            )
            .ok()?;
        }
        buffer.truncate(size as usize);
        let path = String::from_utf16_lossy(&buffer);
        let exe_name = PathBuf::from(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_ascii_lowercase())
            .unwrap_or_default();
        Some((exe_name, Some(path)))
    })();

    unsafe {
        let _ = CloseHandle(handle);
    }

    result
}

fn is_calliop_window(window: &ActiveWindow) -> bool {
    if window.exe_name.eq_ignore_ascii_case("calliop.exe")
        || window.exe_name.eq_ignore_ascii_case("calliop")
    {
        return true;
    }

    if window
        .process_id
        .is_some_and(|pid| pid == std::process::id())
    {
        return true;
    }

    if let Some(path) = &window.exe_path {
        if std::env::current_exe()
            .ok()
            .is_some_and(|own_exe| own_exe.to_string_lossy().eq_ignore_ascii_case(path))
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_active_window_smoke() {
        let _ = super::get_active_window();
    }

    #[test]
    fn is_calliop_window_matches_exe_name() {
        let window = ActiveWindow {
            title: String::new(),
            exe_name: "calliop.exe".into(),
            exe_path: None,
            process_id: None,
        };
        assert!(super::is_calliop_window(&window));
    }

    #[test]
    fn is_calliop_window_matches_current_pid() {
        let window = ActiveWindow {
            title: String::new(),
            exe_name: String::new(),
            exe_path: None,
            process_id: Some(std::process::id()),
        };
        assert!(super::is_calliop_window(&window));
    }

    #[test]
    fn is_calliop_window_external_process_is_false() {
        let window = ActiveWindow {
            title: "Chrome".into(),
            exe_name: "chrome.exe".into(),
            exe_path: None,
            process_id: Some(4242),
        };
        assert!(!super::is_calliop_window(&window));
    }
}
