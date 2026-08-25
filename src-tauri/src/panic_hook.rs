use std::{
    fs::OpenOptions,
    io::Write,
    panic,
    time::{SystemTime, UNIX_EPOCH},
};

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
        let path = std::env::temp_dir().join("z-com-panic.log");
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(
                file,
                "[{timestamp}] thread={thread_name} location={location} panic={payload}"
            );
        }

        default_hook(info);
    }));
}
