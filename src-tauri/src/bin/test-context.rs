use calliop_lib::app_context::{get_active_window, resolve_tone};
use calliop_prompt::ToneProfile;

fn main() {
    match get_active_window() {
        Some(window) => {
            println!("Title: {}", window.title);
            println!("Exe: {}", window.exe_name);
            if let Some(ref path) = window.exe_path {
                println!("Path: {path}");
            }
            let tone = resolve_tone(&window, &[]);
            println!("Tone (no rules): {}", tone.as_str());
        }
        None => {
            println!("No foreground window detected.");
        }
    }

    let _ = ToneProfile::Default;
}
