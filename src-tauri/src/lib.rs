mod audio;
mod hotkey;
mod inject;
mod llm;
mod pipeline;
mod store;
mod stt;

use tauri::tray::TrayIconBuilder;

/// Returns registered core module names (Phase 0 wiring check).
pub fn registered_modules() -> [&'static str; 7] {
    [
        audio::module_name(),
        stt::module_name(),
        llm::module_name(),
        inject::module_name(),
        hotkey::module_name(),
        store::module_name(),
        pipeline::module_name(),
    ]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let _modules = registered_modules();

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Calliop")
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod integration_tests {
    use super::registered_modules;

    #[test]
    fn all_modules_are_wired() {
        assert_eq!(
            registered_modules(),
            ["audio", "stt", "llm", "inject", "hotkey", "store", "pipeline",]
        );
    }
}
