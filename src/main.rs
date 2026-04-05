#[cfg(target_os = "macos")]
mod actions;
#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod menu;
#[cfg(target_os = "macos")]
mod timer;
#[cfg(target_os = "macos")]
mod windows;

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("BreakTime only supports macOS.");
}
