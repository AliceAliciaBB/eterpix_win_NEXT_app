// startup.rs - Windowsスタートアップ登録管理

const APP_NAME: &str = "EterPixVRCUploader";
const REG_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// スタートアップに登録されているか確認
#[cfg(target_os = "windows")]
pub fn is_startup_registered() -> bool {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(REG_KEY) {
        return key.get_value::<String, _>(APP_NAME).is_ok();
    }
    false
}

/// スタートアップに登録
#[cfg(target_os = "windows")]
pub fn register_startup(exe_path: &str) -> bool {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(REG_KEY, KEY_SET_VALUE) {
        let cmd = format!("\"{}\" --minimized", exe_path);
        return key.set_value(APP_NAME, &cmd).is_ok();
    }
    false
}

/// スタートアップから解除
#[cfg(target_os = "windows")]
pub fn unregister_startup() -> bool {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(REG_KEY, KEY_SET_VALUE) {
        return key.delete_value(APP_NAME).is_ok();
    }
    true // 未登録も成功扱い
}

// Windows以外のスタブ
#[cfg(not(target_os = "windows"))]
pub fn is_startup_registered() -> bool { false }
#[cfg(not(target_os = "windows"))]
pub fn register_startup(_exe_path: &str) -> bool { false }
#[cfg(not(target_os = "windows"))]
pub fn unregister_startup() -> bool { false }
