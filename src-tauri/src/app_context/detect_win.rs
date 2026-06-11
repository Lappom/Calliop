//! Windows foreground window detection via Win32 APIs.

use std::path::PathBuf;

use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND, MAX_PATH};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
};

use super::ActiveWindow;

pub fn get_active_window() -> Option<ActiveWindow> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return None;
    }

    let title = read_window_title(hwnd)?;
    let process_id = window_process_id(hwnd);
    let (exe_name, exe_path) = read_process_info(hwnd).unwrap_or((String::new(), None));
    if is_own_process(&exe_path, process_id) {
        return None;
    }

    Some(ActiveWindow {
        title,
        exe_name,
        exe_path,
    })
}

fn read_window_title(hwnd: HWND) -> Option<String> {
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length <= 0 {
        return Some(String::new());
    }

    let mut buffer = vec![0_u16; (length as usize) + 1];
    let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    if copied <= 0 {
        return None;
    }
    buffer.truncate(copied as usize);
    Some(String::from_utf16_lossy(&buffer))
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

fn is_own_process(exe_path: &Option<String>, process_id: Option<u32>) -> bool {
    if process_id == Some(std::process::id()) {
        return true;
    }
    let Some(path) = exe_path else {
        return false;
    };
    std::env::current_exe()
        .ok()
        .is_some_and(|own_exe| own_exe.to_string_lossy().eq_ignore_ascii_case(path))
}

#[cfg(test)]
mod tests {
    #[test]
    fn get_active_window_smoke() {
        // May return None in headless CI; should not panic.
        let _ = super::get_active_window();
    }

    #[test]
    fn is_own_process_matches_current_pid() {
        assert!(super::is_own_process(&None, Some(std::process::id())));
    }

    #[test]
    fn is_own_process_without_pid_or_path_is_false() {
        assert!(!super::is_own_process(&None, None));
    }
}
