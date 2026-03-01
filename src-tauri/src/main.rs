// メインエントリーポイント (Tauri v2)
// lib::run() に処理を委譲する

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    eterpix_vrc_uploader_lib::run()
}
