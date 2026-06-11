//! Helpers for spawning child processes without flashing a console on Windows.

/// Prevent a child process from allocating or attaching to a visible console window.
#[cfg_attr(not(windows), allow(unused_variables))]
pub fn hide_console(cmd: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
}
