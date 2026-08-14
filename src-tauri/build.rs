fn main() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
    let build_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    println!("cargo:rustc-env=Z_COM_BUILD_TIMESTAMP={build_timestamp}");
    tauri_build::build()
}
