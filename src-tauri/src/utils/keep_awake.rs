//! Prevent the system from sleeping while downloads are active (Windows).
//!
//! `SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED)` requests the
//! system to stay awake; the `ES_CONTINUOUS` flag makes a single call sticky
//! until the next call with `ES_CONTINUOUS` and no other flags, so no periodic
//! re-arming thread is needed. Display-only sleep (`ES_DISPLAY_REQUIRED`) is
//! intentionally NOT set — the monitor may still turn off.
//!
//! On non-Windows targets every function is a safe no-op (guarded by
//! `#[cfg(windows)]`), so the rest of the codebase calls these unconditionally.

use std::sync::atomic::{AtomicBool, Ordering};

/// Whether keep-awake is currently active in this process.
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Ask the system to stay awake. Idempotent — a second call while already
/// active is a no-op.
pub fn enable() {
    if ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Power::{
            ES_CONTINUOUS, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
        };
        let _ = SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
    }
    tracing::info!("keep-awake: enabled (prevent system sleep while downloading)");
}

/// Clear the keep-awake request, letting the system sleep again. Idempotent.
pub fn disable() {
    if !ACTIVE.swap(false, Ordering::SeqCst) {
        return;
    }
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Power::{ES_CONTINUOUS, SetThreadExecutionState};
        let _ = SetThreadExecutionState(ES_CONTINUOUS);
    }
    tracing::info!("keep-awake: disabled");
}
