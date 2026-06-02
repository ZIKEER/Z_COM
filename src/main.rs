mod version;
mod app_core;
mod io;
mod ui;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    env_logger::init();
    log::info!("{} v{} starting", version::APP_NAME, version::VERSION);
    let app = AppWindow::new()?;
    app.run()
}
