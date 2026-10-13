use std::{
    fs::OpenOptions,
    io::Write,
    panic,
    path::PathBuf,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

static LOG_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();

pub fn set_log_directory(directory: PathBuf) {
    let _ = LOG_DIRECTORY.set(directory);
}

pub fn init() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("unknown");

        let payload = if let Some(message) = info.payload().downcast_ref::<&str>() {
            *message
        } else if let Some(message) = info.payload().downcast_ref::<String>() {
            message.as_str()
        } else {
            "未知 panic 内容"
        };

        let location = info
            .location()
            .map(|location| {
                format!(
                    "{}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                )
            })
            .unwrap_or_else(|| "未知位置".to_string());
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        let directory = LOG_DIRECTORY.get().cloned().unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(|path| path.join("logs")))
                .unwrap_or_else(|| std::env::temp_dir().join("z-com-logs"))
        });
        let _ = std::fs::create_dir_all(&directory);
        let date = chrono::Local::now().format("%Y-%m-%d");
        let path = directory.join(format!("log_{date}.txt"));
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(
                file,
                "[{timestamp}] thread={thread_name} location={location} panic={payload}"
            );
        }

        default_hook(info);
    }));
}
