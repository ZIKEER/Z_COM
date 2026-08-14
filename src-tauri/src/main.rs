// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Some(result) = z_com_lib::run_update_mode() {
        if let Err(error) = result {
            eprintln!("Z_COM 更新失败: {error}");
            std::process::exit(1);
        }
        return;
    }
    z_com_lib::run()
}
