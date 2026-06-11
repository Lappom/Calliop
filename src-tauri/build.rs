fn main() {
    println!(
        "cargo:rustc-env=CALLIOP_TARGET_TRIPLE={}",
        std::env::var("TARGET").expect("TARGET must be set by Cargo")
    );
    tauri_build::build()
}
