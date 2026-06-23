use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use windows::{
    core::HSTRING,
    Media::SpeechRecognition::{
        SpeechContinuousRecognitionCompletedEventArgs,
        SpeechContinuousRecognitionResultGeneratedEventArgs,
        SpeechContinuousRecognitionSession,
        SpeechRecognitionResultStatus,
        SpeechRecognitionScenario,
        SpeechRecognitionTopicConstraint,
        SpeechRecognizer,
    },
    Win32::Foundation::{HWND, LPARAM, WPARAM},
    Win32::UI::WindowsAndMessaging::PostMessageW,
};

extern crate windows_core;

// Must match WM_USER constants in main.rs (WM_USER = 0x0400)
pub const WM_VOICE_WAKEWORD: u32 = 0x0400 + 100;
pub const WM_VOICE_QUERY_READY: u32 = 0x0400 + 101;

// Wrap HWND as usize so it can cross thread boundaries safely
#[derive(Clone, Copy)]
struct HwndPtr(usize);
unsafe impl Send for HwndPtr {}
unsafe impl Sync for HwndPtr {}
impl HwndPtr {
    fn hwnd(self) -> HWND {
        HWND(self.0 as *mut std::ffi::c_void)
    }
}

// Wake phrases (lowercase). Dictation transcribes free-form speech, so we also
// include a few common mis-hearings of "search".
const WAKE_PHRASES: &[&str] = &[
    "open search", "hey search", "hey speech", "hey open search",
    "open surge", "open serch", "open sersh", "hi search",
];

// After a bare wake word ("open search" with nothing after it), wait briefly and
// treat the next utterance as the query even without a wake word.
static EXPECT_QUERY_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── Simple file logger for diagnostics (voice_log.txt in cwd) ────────────────
fn log_voice(msg: String) {
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("voice_log.txt")
    {
        use std::io::Write;
        let _ = writeln!(file, "[{}] {}", now_ms(), msg);
    }
}

// ── Always-on voice listener ──────────────────────────────────────────────────

/// Spawn ONE background thread running a continuous dictation session that never
/// stops. It detects a wake phrase and the query in a single spoken utterance,
/// e.g. "open search what is the time" → query "what is the time".
///
/// This must be called exactly once at startup. The session self-restarts if it
/// ever completes (silence timeout, transient error).
pub fn start_wake_word_listener(hwnd: HWND) {
    let h = HwndPtr(hwnd.0 as usize);
    std::thread::spawn(move || {
        log_voice("listener thread: starting".into());
        let _ = unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
        };
        loop {
            match run_listen_loop(h) {
                Ok(()) => {
                    log_voice("listen loop ended; rebuilding now".into());
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                Err(e) => {
                    log_voice(format!("listen loop error: {:?}; retry in 2s", e));
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            }
        }
    });
}

fn run_listen_loop(h: HwndPtr) -> windows_core::Result<()> {
    log_voice("creating dictation recognizer".into());
    let recognizer = SpeechRecognizer::new()?;

    // The recognizer gives up after InitialSilenceTimeout (default ~5s) if no speech is
    // heard, completing with TimeoutExceeded. For an always-on listener we never want
    // that, so push the silence/babble timeouts way out. (Ignore errors — if a value is
    // clamped, the session-rebuild fallback still keeps us alive.)
    let one_hour = windows::Foundation::TimeSpan { Duration: 36_000_000_000 };
    if let Ok(timeouts) = recognizer.Timeouts() {
        let _ = timeouts.SetInitialSilenceTimeout(one_hour);
        let _ = timeouts.SetBabbleTimeout(one_hour);
    }

    let constraint = SpeechRecognitionTopicConstraint::Create(
        SpeechRecognitionScenario::Dictation,
        &HSTRING::from("dictation"),
    )?;
    recognizer.Constraints()?.Append(&constraint)?;
    recognizer.CompileConstraintsAsync()?.get()?;
    log_voice("dictation constraints compiled".into());

    let session: SpeechContinuousRecognitionSession =
        recognizer.ContinuousRecognitionSession()?;

    // Fire on every recognized phrase
    let result_handler = windows::Foundation::TypedEventHandler::<
        SpeechContinuousRecognitionSession,
        SpeechContinuousRecognitionResultGeneratedEventArgs,
    >::new(move |_s, args| {
        if let Some(args) = args {
            let result = args.Result()?;
            if result.Status()? == SpeechRecognitionResultStatus::Success {
                let text = result.Text()?.to_string();
                if !text.trim().is_empty() {
                    log_voice(format!("recognized: '{}'", text));
                    handle_recognized(h, &text);
                }
            }
        }
        Ok(())
    });
    session.ResultGenerated(&result_handler)?;

    // Default continuous dictation auto-stops after ~20s of silence. Push that timeout
    // way out so the session stays alive between commands (always-on listening).
    let one_hour = windows::Foundation::TimeSpan { Duration: 36_000_000_000 };
    let _ = session.SetAutoStopSilenceTimeout(one_hour);

    // If the session ever completes (error/timeout), rebuild a fresh recognizer rather
    // than reusing this one — reusing a completed session can hang on StartAsync.
    let completed = Arc::new(AtomicBool::new(false));
    let c = completed.clone();
    let completed_handler = windows::Foundation::TypedEventHandler::<
        SpeechContinuousRecognitionSession,
        SpeechContinuousRecognitionCompletedEventArgs,
    >::new(move |_s, args| {
        if let Some(args) = args {
            if let Ok(status) = args.Status() {
                log_voice(format!("session completed: status={:?}", status));
            }
        }
        c.store(true, Ordering::SeqCst);
        Ok(())
    });
    session.Completed(&completed_handler)?;

    session.StartAsync()?.get()?;
    log_voice("dictation session started; listening".into());

    loop {
        std::thread::sleep(std::time::Duration::from_millis(150));
        if completed.swap(false, Ordering::SeqCst) {
            // Try to restart the SAME session immediately (fast, ~no gap). Only fall back
            // to a full recognizer rebuild if that fails.
            log_voice("session completed; restarting".into());
            match session.StartAsync() {
                Ok(op) => {
                    if op.get().is_err() {
                        return Ok(());
                    }
                    log_voice("session restarted in place".into());
                }
                Err(_) => return Ok(()),
            }
        }
    }
}

/// Parse a recognized utterance: find the wake phrase, extract the query after it,
/// and post it to the launcher. Handles both single-utterance and bare-wake flows.
fn handle_recognized(h: HwndPtr, text: &str) {
    let lower = text.to_lowercase();

    // 1) Wake phrase present anywhere in the utterance
    for wake in WAKE_PHRASES {
        if let Some(pos) = lower.find(wake) {
            let after = lower[pos + wake.len()..]
                .trim_start_matches(|c: char| {
                    c == ',' || c == '.' || c == '?' || c == '!' || c.is_whitespace()
                })
                .trim()
                .to_string();
            if after.is_empty() {
                // Bare wake word — open empty, expect a follow-up utterance as the query
                EXPECT_QUERY_UNTIL_MS.store(now_ms() + 7000, Ordering::SeqCst);
                post_wake(h, String::new());
            } else {
                EXPECT_QUERY_UNTIL_MS.store(0, Ordering::SeqCst);
                post_wake(h, after);
            }
            return;
        }
    }

    // 2) No wake phrase, but we're within the follow-up window from a bare wake
    if now_ms() < EXPECT_QUERY_UNTIL_MS.load(Ordering::SeqCst) {
        EXPECT_QUERY_UNTIL_MS.store(0, Ordering::SeqCst);
        let q = lower.trim().to_string();
        if !q.is_empty() {
            post_wake(h, q);
        }
    }
}

fn post_wake(h: HwndPtr, query: String) {
    log_voice(format!("posting wakeword; query='{}'", query));
    let ptr = Box::into_raw(Box::new(query)) as isize;
    unsafe {
        if PostMessageW(h.hwnd(), WM_VOICE_WAKEWORD, WPARAM(1), LPARAM(ptr)).is_err() {
            // Window gone — reclaim the box to avoid a leak
            let _ = Box::from_raw(ptr as *mut String);
        }
    }
}
