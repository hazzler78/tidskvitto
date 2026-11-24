use std::time::Duration;

/// Ett stickprov av det främsta fönstret. Vi anropar inte x-wins
/// `get_browser_url`: adressen kan innehålla mejl-id och behövs inte.
pub struct Sample {
    pub app_name: String,
    pub window_title: String,
    pub idle_for: Option<Duration>,
    pub idle_supported: bool,
}

pub fn sample() -> Sample {
    let (app_name, window_title) = foreground();
    let (idle_for, idle_supported) = idle_for();
    Sample {
        app_name,
        window_title,
        idle_for,
        idle_supported,
    }
}

pub fn screen_recording() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        if unsafe { CGPreflightScreenCaptureAccess() } {
            "granted"
        } else {
            "denied"
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        "not_required"
    }
}

pub fn request_screen_recording() -> Result<&'static str, String> {
    #[cfg(target_os = "macos")]
    {
        let _ = unsafe { CGRequestScreenCaptureAccess() };
        Ok(screen_recording())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok("not_required")
    }
}

fn foreground() -> (String, String) {
    let result = std::panic::catch_unwind(|| x_win::get_active_window());
    match result {
        Ok(Ok(info)) => {
            let name = if info.info.name.trim().is_empty() {
                info.info.exec_name
            } else {
                info.info.name
            };
            (name.trim().to_string(), info.title.trim().to_string())
        }
        _ => (String::new(), String::new()),
    }
}

fn idle_for() -> (Option<Duration>, bool) {
    match idle_millis() {
        Some(ms) => (Some(Duration::from_millis(ms)), true),
        None => (None, false),
    }
}

#[cfg(target_os = "windows")]
fn idle_millis() -> Option<u64> {
    #[repr(C)]
    struct LastInputInfo {
        cb_size: u32,
        dw_time: u32,
    }
    #[link(name = "user32")]
    extern "system" {
        fn GetLastInputInfo(plii: *mut LastInputInfo) -> i32;
        fn GetTickCount() -> u32;
    }
    unsafe {
        let mut info = LastInputInfo {
            cb_size: std::mem::size_of::<LastInputInfo>() as u32,
            dw_time: 0,
        };
        if GetLastInputInfo(&mut info) == 0 {
            return None;
        }
        Some(GetTickCount().wrapping_sub(info.dw_time) as u64)
    }
}

#[cfg(target_os = "macos")]
fn idle_millis() -> Option<u64> {
    // 1 = kCGEventSourceStateHIDSystemState, !0 = kCGAnyInputEventType.
    let seconds = unsafe { CGEventSourceSecondsSinceLastEventType(1, u32::MAX) };
    if !seconds.is_finite() || seconds < 0.0 {
        None
    } else {
        Some((seconds * 1000.0) as u64)
    }
}

#[cfg(target_os = "linux")]
fn idle_millis() -> Option<u64> {
    let output = std::process::Command::new("xprintidle").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    text.trim().parse::<u64>().ok()
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
    fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
}
