use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        UI::Shell::ShellExecuteW,
        UI::WindowsAndMessaging::SW_SHOWNORMAL,
        Foundation::HWND,
    },
};

pub fn launch(cmd: &str) {
    let cmd = cmd.trim();
    if cmd.is_empty() { return; }

    // Map breadcrumbs and non-executable control panel applets to valid commands
    let cmd = match cmd {
        "Windows Defender Firewall > Customize settings > Private network settings" |
        "Windows Defender Firewall > Customize settings > Public network settings" => "control.exe /name Microsoft.WindowsFirewall",
        "System > Set priority notifications > Calls and reminders > Show incoming calls" |
        "System > Set priority notifications > Calls and reminders > Show reminders" => "ms-settings:notifications",
        "inetcpl.cpl" => "control.exe inetcpl.cpl",
        _ => cmd,
    };

    // ── VS Code direct line number opening ──────────────────────────────
    if let Some(rest) = cmd.strip_prefix("vscode:") {
        if let Some(last_colon) = rest.rfind(':') {
            let file_path = &rest[..last_colon];
            let line_number = &rest[last_colon + 1..];
            let _ = Command::new("cmd")
                .args(["/c", &format!("code -g \"{file_path}\":{line_number}")])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .spawn();
        }
        return;
    }

    // ── Window switcher: Focus window by HWND ──────────────────────────────
    if let Some(hwnd_str) = cmd.strip_prefix("window:") {
        if let Ok(hwnd_val) = hwnd_str.trim().parse::<isize>() {
            let target_hwnd = windows::Win32::Foundation::HWND(hwnd_val as *mut std::ffi::c_void);
            unsafe {
                use windows::Win32::UI::WindowsAndMessaging::{
                    ShowWindow, SetForegroundWindow, SW_RESTORE, IsIconic
                };
                use windows::Win32::UI::Input::KeyboardAndMouse::SetActiveWindow;
                if IsIconic(target_hwnd).as_bool() {
                    let _ = ShowWindow(target_hwnd, SW_RESTORE);
                }
                let _ = SetForegroundWindow(target_hwnd);
                let _ = SetActiveWindow(target_hwnd);
            }
        }
        return;
    }

    // ── Kill process by PID ────────────────────────────────────────────────
    if let Some(pid) = cmd.strip_prefix("kill:") {
        let _ = Command::new("taskkill")
            .args(["/F", "/PID", pid.trim()])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn();
        return;
    }

    // ── Action commands ────────────────────────────────────────────────────
    if let Some(action) = cmd.strip_prefix("action:") {
        handle_action(action);
        return;
    }

    let cmd_lower = cmd.to_lowercase();

    // ── ChatGPT: open URL (fills box via ?q=) then auto-submit with Enter ─
    if cmd.starts_with("https://chatgpt.com/?q=") {
        let cmd_wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            ShellExecuteW(
                HWND::default(),
                w!("open"),
                PCWSTR(cmd_wide.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
        }
        // Spawn a thread: poll for the ChatGPT browser window to appear,
        // then focus it and send Enter to submit the filled prompt.
        std::thread::spawn(|| {
            let _ = Command::new("powershell")
                .args([
                    "-WindowStyle", "Hidden",
                    "-Command",
                    "Add-Type -AssemblyName Microsoft.VisualBasic; \
                     Add-Type -AssemblyName System.Windows.Forms; \
                     for ($i = 0; $i -lt 30; $i++) { \
                         $proc = Get-Process | Where-Object { $_.MainWindowTitle -match 'ChatGPT|OpenAI' -and $_.ProcessName -notmatch 'notepad|code' } | Select-Object -First 1; \
                         if ($proc) { \
                             for ($j = 0; $j -lt 4; $j++) { \
                                 $activated = $false; \
                                 try { \
                                     [Microsoft.VisualBasic.Interaction]::AppActivate($proc.Id); \
                                     $activated = $true; \
                                 } catch { \
                                     try { \
                                         [Microsoft.VisualBasic.Interaction]::AppActivate($proc.MainWindowTitle); \
                                         $activated = $true; \
                                     } catch {} \
                                 } \
                                 if ($activated) { \
                                     Start-Sleep -Milliseconds 200; \
                                     [System.Windows.Forms.SendKeys]::SendWait('{ENTER}'); \
                                 } \
                                 Start-Sleep -Milliseconds 1500; \
                             } \
                             break; \
                         } \
                         Start-Sleep -Milliseconds 500; \
                     }",
                ])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .spawn();
        });
        return;
    }

    if cmd.starts_with("http://") || cmd.starts_with("https://") || cmd_lower.ends_with(".lnk") || std::path::Path::new(cmd).exists() {
        let cmd_wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            ShellExecuteW(
                HWND::default(),
                w!("open"),
                PCWSTR(cmd_wide.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
        }
        return;
    }

    let _ = if cmd.starts_with("ms-settings:") || cmd.starts_with("shell:") {
        Command::new("explorer.exe").arg(cmd).spawn()
    } else if let Some(rest) = cmd.strip_prefix("control.exe") {
        let mut c = Command::new("control.exe");
        for arg in rest.split_whitespace() {
            c.arg(arg);
        }
        c.spawn()
    } else if cmd.ends_with(".msc") {
        Command::new("mmc.exe").arg(cmd).spawn()
    } else {
        let mut parts = cmd.split_whitespace();
        if let Some(exe) = parts.next() {
            let mut c = Command::new(exe);
            for arg in parts {
                c.arg(arg);
            }
            match c.spawn() {
                Ok(child) => Ok(child),
                Err(_) => {
                    let cmd_wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
                    unsafe {
                        let _ = ShellExecuteW(
                            HWND::default(),
                            w!("open"),
                            PCWSTR(cmd_wide.as_ptr()),
                            PCWSTR::null(),
                            PCWSTR::null(),
                            SW_SHOWNORMAL,
                        );
                    }
                    Err(std::io::Error::new(std::io::ErrorKind::Other, "fallback to ShellExecuteW"))
                }
            }
        } else {
            return;
        }
    };
}

fn handle_action(action: &str) {
    match action {
        "lock" => {
            unsafe {
                let _ = windows::Win32::System::Shutdown::LockWorkStation();
            }
        }
        "shutdown" => {
            let _ = Command::new("shutdown").args(["/s", "/t", "0"]).spawn();
        }
        "restart" => {
            let _ = Command::new("shutdown").args(["/r", "/t", "0"]).spawn();
        }
        "sleep" => {
            unsafe {
                let _ = windows::Win32::System::Power::SetSuspendState(false, false, false);
            }
        }
        "recycle" => {
            unsafe {
                use windows::Win32::UI::Shell::{SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI};
                let _ = SHEmptyRecycleBinW(
                    HWND::default(),
                    PCWSTR::null(),
                    SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI,
                );
            }
        }
        "flushdns" => {
            let _ = Command::new("cmd")
                .args(["/c", "ipconfig /flushdns"])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .spawn();
        }
        "envvars" => {
            let _ = Command::new("rundll32.exe")
                .args(["sysdm.cpl,EditEnvironmentVariables"])
                .spawn();
        }
        "clearclip" => {
            unsafe {
                use windows::Win32::System::DataExchange::{OpenClipboard, EmptyClipboard, CloseClipboard};
                if OpenClipboard(HWND::default()).is_ok() {
                    let _ = EmptyClipboard();
                    let _ = CloseClipboard();
                }
            }
        }
        "hosts" => {
            let hosts = r"C:\Windows\System32\drivers\etc\hosts";
            let _ = Command::new("notepad.exe").arg(hosts).spawn();
        }
        "restart_explorer" => {
            let _ = Command::new("cmd")
                .args(["/c", "taskkill /F /IM explorer.exe & timeout /t 2 /nobreak >nul & start explorer.exe"])
                .creation_flags(0x08000000)
                .spawn();
        }
        "volume_up" => {
            let _ = Command::new("powershell")
                .args([
                    "-WindowStyle", "Hidden",
                    "-Command",
                    "$wshShell = New-Object -ComObject WScript.Shell; 1..5 | ForEach-Object { $wshShell.SendKeys([char]175) }",
                ])
                .creation_flags(0x08000000)
                .spawn();
        }
        "volume_down" => {
            let _ = Command::new("powershell")
                .args([
                    "-WindowStyle", "Hidden",
                    "-Command",
                    "$wshShell = New-Object -ComObject WScript.Shell; 1..5 | ForEach-Object { $wshShell.SendKeys([char]174) }",
                ])
                .creation_flags(0x08000000)
                .spawn();
        }
        "toggle_mute" => {
            let _ = Command::new("powershell")
                .args([
                    "-WindowStyle", "Hidden",
                    "-Command",
                    "$wshShell = New-Object -ComObject WScript.Shell; $wshShell.SendKeys([char]173)",
                ])
                .creation_flags(0x08000000)
                .spawn();
        }
        "toggle_bluetooth" => {
            let _ = Command::new("powershell")
                .args([
                    "-WindowStyle", "Hidden",
                    "-Command",
                    "Get-Service bthserv | ForEach-Object { if ($_.Status -eq 'Running') { Stop-Service -Name 'bthserv' -Force } else { Start-Service -Name 'bthserv' } }",
                ])
                .creation_flags(0x08000000)
                .spawn();
        }
        "toggle_wifi" => {
            let _ = Command::new("powershell")
                .args([
                    "-WindowStyle", "Hidden",
                    "-Command",
                    "$adapter = Get-NetAdapter -Name 'Wi-Fi' -ErrorAction SilentlyContinue; if ($adapter) { if ($adapter.Status -eq 'Up') { Disable-NetAdapter -Name 'Wi-Fi' -Confirm:$false } else { Enable-NetAdapter -Name 'Wi-Fi' -Confirm:$false } } else { Start-Process 'ms-settings:network-wifi' }",
                ])
                .creation_flags(0x08000000)
                .spawn();
        }
        "ipconfig" => {
            let _ = Command::new("cmd")
                .args(["/c", "start cmd /k ipconfig /all"])
                .creation_flags(0x08000000)
                .spawn();
        }
        "ip_release" => {
            let _ = Command::new("cmd")
                .args(["/c", "ipconfig /release"])
                .creation_flags(0x08000000)
                .spawn();
        }
        "ip_renew" => {
            let _ = Command::new("cmd")
                .args(["/c", "ipconfig /renew"])
                .creation_flags(0x08000000)
                .spawn();
        }
        "wifi_password" => {
            let _ = Command::new("powershell")
                .args([
                    "-WindowStyle", "Hidden",
                    "-Command",
                    "Start-Process cmd -ArgumentList '/k', 'netsh', 'wlan', 'show', 'profiles' -Wait; $profiles = (netsh wlan show profiles) -join \"`n\"; Start-Process cmd -ArgumentList '/k', 'echo', 'Run:', 'netsh wlan show profile name=\"PROFILE\" key=clear'",
                ])
                .creation_flags(0x08000000)
                .spawn();
        }
        "kill_process_prompt" => {
            let _ = Command::new("cmd")
                .args(["/c", "start cmd /k \"echo Kill a process by name && set /p pname=Process name: && taskkill /F /IM %pname%\""])
                .creation_flags(0x08000000)
                .spawn();
        }
        "eject_cd" => {
            let _ = Command::new("powershell")
                .args([
                    "-WindowStyle", "Hidden",
                    "-Command",
                    "$wmp = New-Object -ComObject WMPlayer.OCX; $wmp.cdromCollection.Item(0).Eject()",
                ])
                .creation_flags(0x08000000)
                .spawn();
        }
        folder if folder.starts_with("folder:") => {
            let which = &folder[7..];
            let path = match which {
                "downloads" => get_known_folder_path(&windows::Win32::UI::Shell::FOLDERID_Downloads),
                "desktop"   => get_known_folder_path(&windows::Win32::UI::Shell::FOLDERID_Desktop),
                "documents" => get_known_folder_path(&windows::Win32::UI::Shell::FOLDERID_Documents),
                "pictures"  => get_known_folder_path(&windows::Win32::UI::Shell::FOLDERID_Pictures),
                "music"     => get_known_folder_path(&windows::Win32::UI::Shell::FOLDERID_Music),
                "videos"    => get_known_folder_path(&windows::Win32::UI::Shell::FOLDERID_Videos),
                "temp"      => std::env::var("TEMP").ok(),
                _ => None,
            };
            if let Some(p) = path {
                let _ = Command::new("explorer.exe").arg(p).spawn();
            }
        }
        _ => {}
    }
}

fn get_known_folder_path(folder_id: &windows::core::GUID) -> Option<String> {
    unsafe {
        use windows::Win32::UI::Shell::{SHGetKnownFolderPath, KF_FLAG_DEFAULT};
        use windows::Win32::Foundation::HANDLE;
        let result = SHGetKnownFolderPath(folder_id, KF_FLAG_DEFAULT, HANDLE::default()).ok()?;
        let mut len = 0;
        while *result.0.add(len) != 0 { len += 1; }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(result.0, len));
        windows::Win32::System::Com::CoTaskMemFree(Some(result.0 as *const _));
        Some(s)
    }
}
