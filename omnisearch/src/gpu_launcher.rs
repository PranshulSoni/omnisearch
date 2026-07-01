// GPU launcher spike (branch: gpu-test).
//
// M1 proved the visual foundation (frameless + transparent + always-on-top,
// vsync'd femtovg rendering). M2 (this file) proves the RISKY part: the Win32
// glue re-plumbed onto Slint's winit-created HWND —
//   * Alt+Space global hotkey (summon / toggle)
//   * focus-grab on summon (AttachThreadInput + SetForegroundWindow)
//   * dismiss on focus loss (WM_ACTIVATE / WA_INACTIVE) and on Esc
//   * tray icon (Shell_NotifyIconW) with left-click toggle + right-click menu
//
// Still a SEPARATE binary; main.rs (the GDI launcher) is untouched.
//
// Design notes (why it's shaped like this):
//  - We subclass the HWND with SetWindowLongPtrW(GWLP_WNDPROC) and CHAIN to
//    winit's original proc, instead of comctl32 SetWindowSubclass — fewer feature
//    deps, and winit keeps working as long as we always chain unhandled messages.
//  - Real visibility is toggled with raw ShowWindow(SW_HIDE / SetWindowPos show).
//    We never call Slint's hide(): that could destroy/recreate the HWND (breaking
//    the subclass + tray + hotkey) and could make the event loop auto-quit as the
//    "last window". From winit's view the window is always shown; only the OS hides it.
//  - The subclass proc runs in winit's message pump; it ONLY flips atomics. A
//    slint::Timer on the UI thread reads them and performs the window ops (so we
//    have the ComponentHandle for request_redraw and avoid reentrancy with winit).

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

const HOTKEY_ID: i32 = 1;
const WM_TRAYICON: u32 = 0x0400 + 9; // WM_USER + 9, mirrors main.rs
const TRAY_UID: u32 = 1;
const MENU_OPEN: usize = 1;
const MENU_EXIT: usize = 2;
// Ignore focus-loss dismissal within this window after a programmatic show, so the
// summon sequence (show -> foreground-grab) can't race into an immediate hide.
const SHOW_GUARD_MS: u32 = 350;

static OLD_WNDPROC: AtomicIsize = AtomicIsize::new(0);
static HWND_VAL: AtomicIsize = AtomicIsize::new(0);
static TOGGLE_REQ: AtomicBool = AtomicBool::new(false);
static HIDE_REQ: AtomicBool = AtomicBool::new(false);
static QUIT_REQ: AtomicBool = AtomicBool::new(false);
static LAST_SHOW_TICK: AtomicU32 = AtomicU32::new(0);

slint::slint! {
    struct ResultItem {
        title: string,
        subtitle: string,
        source: string,
    }

    export component LauncherWindow inherits Window {
        no-frame: true;
        background: transparent;
        always-on-top: true;
        width: 720px;
        height: 520px;

        in property <[ResultItem]> results;
        in-out property <int> current-index: 0;
        callback activated(int);
        callback query-edited(string);

        VerticalLayout {
            alignment: start;

            // ── Floating glass panel ────────────────────────────────────────
            Rectangle {
                border-radius: 18px;
                background: #1c1c1eee;          // ~93% opaque dark; corners must show desktop
                border-width: 1px;
                border-color: #ffffff24;
                drop-shadow-color: #000000a6;
                drop-shadow-blur: 34px;
                drop-shadow-offset-y: 10px;

                VerticalLayout {
                    padding: 10px;
                    spacing: 4px;

                    // Search bar
                    Rectangle {
                        height: 52px;
                        Text {
                            x: 12px;
                            y: (parent.height - self.height) / 2;
                            text: "\u{1F50D}";
                            font-size: 19px;
                            color: #808086;
                        }
                        search := TextInput {
                            x: 48px;
                            width: parent.width - 48px - 16px;
                            height: parent.height;
                            vertical-alignment: center;
                            font-size: 19px;
                            color: #f2f2f4;
                            text: "documents";
                            edited => { root.query-edited(self.text); }
                        }
                    }

                    Rectangle { height: 1px; background: #ffffff14; }

                    // Result rows
                    for item[idx] in root.results: Rectangle {
                        height: 52px;
                        border-radius: 10px;
                        background: idx == root.current-index ? #ffffff1f : transparent;

                        TouchArea {
                            clicked => {
                                root.current-index = idx;
                                root.activated(idx);
                            }
                        }

                        // icon placeholder (source tag) — real HICON->Image comes later
                        Rectangle {
                            x: 12px;
                            width: 34px;
                            height: 34px;
                            y: (parent.height - self.height) / 2;
                            border-radius: 8px;
                            background: #3a3a40;
                            Text {
                                width: parent.width;
                                height: parent.height;
                                text: item.source;
                                font-size: 9px;
                                color: #c0c0c6;
                                horizontal-alignment: center;
                                vertical-alignment: center;
                            }
                        }
                        Text {
                            x: 56px;
                            y: 9px;
                            width: parent.width - 56px - 16px;
                            text: item.title;
                            font-size: 15px;
                            color: #f0f0f2;
                            overflow: elide;
                        }
                        Text {
                            x: 56px;
                            y: 28px;
                            width: parent.width - 56px - 16px;
                            text: item.subtitle;
                            font-size: 12px;
                            color: #97979d;
                            overflow: elide;
                        }
                    }

                    // Footer hint
                    Rectangle {
                        height: 22px;
                        Text {
                            x: 12px;
                            y: (parent.height - self.height) / 2;
                            text: "Alt+Space toggle   ·   Esc hide   ·   right-click tray to quit";
                            font-size: 11px;
                            color: #6a6a70;
                        }
                    }
                }
            }
        }
    }
}

/// Extract the Win32 HWND from a shown Slint window (raw-window-handle 0.6).
/// Valid only after the window has been shown + one event-loop iteration.
fn window_hwnd(app: &LauncherWindow) -> Option<HWND> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let slint_wh = app.window().window_handle();
    let rwh = slint_wh.window_handle().ok()?;
    match rwh.as_raw() {
        RawWindowHandle::Win32(h) => Some(HWND(isize::from(h.hwnd) as *mut core::ffi::c_void)),
        _ => None,
    }
}

/// AttachThreadInput trick — lets SetForegroundWindow succeed from a background
/// context (same approach main.rs uses when the launcher is summoned by hotkey).
unsafe fn force_foreground(hwnd: HWND) {
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
    };
    let fore = GetForegroundWindow();
    let fore_tid = GetWindowThreadProcessId(fore, None);
    let my_tid = GetCurrentThreadId();
    if fore_tid != 0 && fore_tid != my_tid {
        let _ = AttachThreadInput(fore_tid, my_tid, true);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(hwnd);
        let _ = AttachThreadInput(fore_tid, my_tid, false);
    } else {
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(hwnd);
    }
}

unsafe fn is_visible(hwnd: HWND) -> bool {
    windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd).as_bool()
}

/// Show, center-near-top, make topmost, and grab foreground — without resizing
/// (SWP_NOSIZE keeps Slint's DPI-correct physical size).
unsafe fn summon(hwnd: HWND) {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, GetWindowRect, SetWindowPos, HWND_TOPMOST, SM_CXSCREEN, SM_CYSCREEN,
        SWP_NOACTIVATE, SWP_NOSIZE, SWP_SHOWWINDOW,
    };
    let mut rc = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rc);
    let w = rc.right - rc.left;
    let sw = GetSystemMetrics(SM_CXSCREEN);
    let sh = GetSystemMetrics(SM_CYSCREEN);
    let x = ((sw - w) / 2).max(0);
    let y = (sh / 6).max(0);
    LAST_SHOW_TICK.store(GetTickCount(), Ordering::SeqCst);
    let _ = SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        x,
        y,
        0,
        0,
        SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
    );
    force_foreground(hwnd);
}

unsafe fn dismiss(hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
    let _ = ShowWindow(hwnd, SW_HIDE);
}

/// Right-click tray menu (Open / Exit). Returns the selected command id (0 = none).
unsafe fn show_tray_menu(hwnd: HWND) -> usize {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, SetForegroundWindow,
        TrackPopupMenu, MF_STRING, TPM_BOTTOMALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    };
    let Ok(hmenu) = CreatePopupMenu() else {
        return 0;
    };
    let open: Vec<u16> = "Open".encode_utf16().chain(std::iter::once(0)).collect();
    let exit: Vec<u16> = "Exit".encode_utf16().chain(std::iter::once(0)).collect();
    let _ = AppendMenuW(hmenu, MF_STRING, MENU_OPEN, PCWSTR(open.as_ptr()));
    let _ = AppendMenuW(hmenu, MF_STRING, MENU_EXIT, PCWSTR(exit.as_ptr()));
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    // Required Win32 quirk so the menu dismisses correctly on outside-click.
    let _ = SetForegroundWindow(hwnd);
    let sel = TrackPopupMenu(
        hmenu,
        TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_RETURNCMD,
        pt.x,
        pt.y,
        0,
        hwnd,
        None,
    );
    let _ = DestroyMenu(hmenu);
    sel.0 as usize
}

unsafe fn add_tray_icon(hwnd: HWND) {
    use windows::Win32::UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NOTIFYICONDATAW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{LoadIconW, IDI_APPLICATION};
    let mut nid = NOTIFYICONDATAW::default();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_UID;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAYICON;
    nid.hIcon = LoadIconW(None, IDI_APPLICATION).unwrap_or_default();
    let tip = "OmniSearch (GPU spike)".encode_utf16().collect::<Vec<u16>>();
    for (i, &c) in tip.iter().enumerate().take(127) {
        nid.szTip[i] = c;
    }
    let _ = Shell_NotifyIconW(NIM_ADD, &nid);
}

unsafe fn remove_tray_icon(hwnd: HWND) {
    use windows::Win32::UI::Shell::{Shell_NotifyIconW, NIM_DELETE, NOTIFYICONDATAW};
    let mut nid = NOTIFYICONDATAW::default();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_UID;
    let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
}

/// Subclass proc chained ahead of winit's. Handles hotkey / tray / Esc / focus-loss,
/// then forwards everything else to the original winit proc. Only flips atomics —
/// the actual window ops run on the UI thread in the poll timer.
unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, WA_INACTIVE, WM_ACTIVATE, WM_HOTKEY, WM_KEYDOWN, WM_LBUTTONUP,
        WM_RBUTTONUP, WNDPROC,
    };

    match msg {
        WM_HOTKEY if wparam.0 as i32 == HOTKEY_ID => {
            TOGGLE_REQ.store(true, Ordering::SeqCst);
            return LRESULT(0);
        }
        WM_KEYDOWN if wparam.0 as u32 == VK_ESCAPE.0 as u32 => {
            if is_visible(hwnd) {
                HIDE_REQ.store(true, Ordering::SeqCst);
                return LRESULT(0); // swallow: Esc dismisses, don't pass to UI
            }
        }
        WM_ACTIVATE => {
            // low word == WA_INACTIVE => we lost activation. Dismiss if visible and
            // we're past the post-show guard window (avoids the summon race).
            if (wparam.0 & 0xFFFF) as u32 == WA_INACTIVE && is_visible(hwnd) {
                let since = GetTickCount().wrapping_sub(LAST_SHOW_TICK.load(Ordering::SeqCst));
                if since > SHOW_GUARD_MS {
                    HIDE_REQ.store(true, Ordering::SeqCst);
                }
            }
        }
        WM_TRAYICON => {
            let evt = lparam.0 as u32;
            if evt == WM_LBUTTONUP {
                TOGGLE_REQ.store(true, Ordering::SeqCst);
            } else if evt == WM_RBUTTONUP {
                match show_tray_menu(hwnd) {
                    MENU_OPEN => TOGGLE_REQ.store(true, Ordering::SeqCst),
                    MENU_EXIT => QUIT_REQ.store(true, Ordering::SeqCst),
                    _ => {}
                }
            }
            return LRESULT(0);
        }
        _ => {}
    }

    let old_val = OLD_WNDPROC.load(Ordering::SeqCst);
    if old_val == 0 {
        use windows::Win32::UI::WindowsAndMessaging::DefWindowProcW;
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let old: WNDPROC = std::mem::transmute::<isize, WNDPROC>(old_val);
    CallWindowProcW(old, hwnd, msg, wparam, lparam)
}

/// One-time Win32 setup once the HWND exists: tool-window ex-style, subclass,
/// register Alt+Space, add tray icon, then hide until summoned.
unsafe fn install_glue(hwnd: HWND) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_NOREPEAT, VK_SPACE,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, ShowWindow, GWLP_WNDPROC, GWL_EXSTYLE, SW_HIDE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, SetWindowPos, SWP_FRAMECHANGED, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    };

    // No taskbar button / Alt-Tab entry (launcher semantics).
    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    let want = ex | (WS_EX_TOOLWINDOW.0 as isize) | (WS_EX_TOPMOST.0 as isize);
    let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, want);

    // Force frame change so ex-style changes take effect.
    let _ = SetWindowPos(
        hwnd,
        HWND(core::ptr::null_mut()),
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
    );

    // Chain our proc ahead of winit's.
    let old = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, subclass_proc as *const () as isize);
    OLD_WNDPROC.store(old, Ordering::SeqCst);

    let _ = RegisterHotKey(
        hwnd,
        HOTKEY_ID,
        HOT_KEY_MODIFIERS(MOD_ALT.0 | MOD_NOREPEAT.0),
        VK_SPACE.0 as u32,
    );

    add_tray_icon(hwnd);

    // Start hidden — summon with Alt+Space. (Raw hide; Slint still thinks it's shown.)
    let _ = ShowWindow(hwnd, SW_HIDE);
    eprintln!("[gpu-launcher] glue installed on hwnd {:?}; hidden, Alt+Space to summon", hwnd);
}

fn main() {
    let app = LauncherWindow::new().expect("create LauncherWindow");

    let rows = [
        ("Quarterly Report.docx", "Documents > Work", "DOC"),
        ("main.rs", "omnisearch > src", "CODE"),
        ("Display Settings", "Settings > System > Display", "SET"),
        ("github.com/slint-ui/slint", "History > Today", "WEB"),
        ("screenshot_2026_07_01.png", "Pictures > OCR text", "OCR"),
        ("Restart Explorer", "System > Process", "ACT"),
    ];
    let items: Vec<ResultItem> = rows
        .iter()
        .map(|(t, s, src)| ResultItem {
            title: (*t).into(),
            subtitle: (*s).into(),
            source: (*src).into(),
        })
        .collect();
    app.set_results(slint::ModelRc::new(slint::VecModel::from(items)));
    app.on_activated(|i| eprintln!("[gpu-launcher] activated row {i}"));
    app.on_query_edited(|q| eprintln!("[gpu-launcher] query: {q}"));

    // One timer drives everything on the UI thread: self-initializes once the HWND
    // is available, then services hotkey / focus-loss / tray requests.
    let weak = app.as_weak();
    let poll = slint::Timer::default();
    poll.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(15),
        move || {
            let Some(app) = weak.upgrade() else { return };

            // Phase 1: install glue once the HWND exists.
            if HWND_VAL.load(Ordering::SeqCst) == 0 {
                if let Some(hwnd) = window_hwnd(&app) {
                    HWND_VAL.store(hwnd.0 as isize, Ordering::SeqCst);
                    unsafe { install_glue(hwnd) };
                }
                return;
            }

            // Phase 2: service requests.
            let hwnd = HWND(HWND_VAL.load(Ordering::SeqCst) as *mut core::ffi::c_void);
            unsafe {
                if TOGGLE_REQ.swap(false, Ordering::SeqCst) {
                    if is_visible(hwnd) {
                        dismiss(hwnd);
                        eprintln!("[gpu-launcher] toggle -> hidden");
                    } else {
                        summon(hwnd);
                        app.window().request_redraw();
                        eprintln!("[gpu-launcher] toggle -> shown + foreground");
                    }
                }
                if HIDE_REQ.swap(false, Ordering::SeqCst) && is_visible(hwnd) {
                    dismiss(hwnd);
                    eprintln!("[gpu-launcher] dismissed (blur/esc)");
                }
                if QUIT_REQ.swap(false, Ordering::SeqCst) {
                    remove_tray_icon(hwnd);
                    let _ = slint::quit_event_loop();
                }
            }
        },
    );

    app.show().expect("show");
    slint::run_event_loop().expect("run event loop");
}
