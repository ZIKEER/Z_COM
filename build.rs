fn main() {
    let build_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
    slint_build::compile("ui/app-window.slint").unwrap();
}
