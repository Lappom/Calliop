use std::env;
use std::thread;
use std::time::Duration;

use calliop_lib::inject::TextInjector;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: test-inject \"text to paste\"");
        eprintln!("Example: cargo run --bin test-inject -- \"Hello world\"");
        std::process::exit(1);
    }

    let text = &args[1];
    println!("Focus the target application within 3 seconds...");
    thread::sleep(Duration::from_secs(3));

    let injector = TextInjector::new()?;
    injector.inject(text)?;
    println!("Injected: {text}");
    Ok(())
}
