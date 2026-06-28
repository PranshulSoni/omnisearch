use std::path::PathBuf;

pub fn set_run_on_startup(enable: bool) {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_str) = exe_path.to_str() {
            set_registry_startup(enable, exe_str);
        }
    }
}

fn set_registry_startup(enable: bool, exe_path: &str) {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    let key_name = "opensearch-os";

    if let Ok((key, _)) = hkcu.create_subkey(path) {
        if enable {
            let _ = key.set_value(key_name, &exe_path);
        } else {
            let _ = key.delete_value(key_name);
        }
    }
}
