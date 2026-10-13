fn main() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
    let build_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    println!("cargo:rustc-env=Z_COM_BUILD_TIMESTAMP={build_timestamp}");
    #[cfg(windows)]
    winres::WindowsResource::new()
        .set_icon("icons/icon.ico")
        .compile()
        .expect("无法编译 Windows 图标资源");
}
