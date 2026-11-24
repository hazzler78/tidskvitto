use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tidskvitto_core::classify;

use crate::platform;
use crate::AppState;

struct OpenSeg {
    id: i64,
    app: String,
    title: String,
    category: String,
    idle: bool,
}

pub fn spawn(state: Arc<AppState>) {
    let _ = thread::Builder::new()
        .name("tidskvitto-tracker".into())
        .spawn(move || {
            let mut open: Option<OpenSeg> = None;
            loop {
                if state.stop.load(Ordering::Relaxed) {
                    break;
                }
                if let Err(err) = tick(&state, &mut open) {
                    eprintln!("tidskvitto: mätningen hoppade över ett prov: {err}");
                }
                for _ in 0..10 {
                    if state.stop.load(Ordering::Relaxed) {
                        return;
                    }
                    thread::sleep(Duration::from_millis(200));
                }
            }
        });
}

fn tick(state: &AppState, open: &mut Option<OpenSeg>) -> Result<(), String> {
    let settings = state.db.load_settings();
    let now = now_ms();
    if !settings.tracking_enabled {
        *open = None;
        set_live(state, "", "", now);
        return Ok(());
    }
    let sample = platform::sample();
    state
        .idle_supported
        .store(sample.idle_supported, Ordering::Relaxed);
    if sample.app_name.is_empty() || is_self(&sample.app_name) {
        *open = None;
        set_live(state, "", "", now);
        return Ok(());
    }
    let category = classify(&sample.app_name, &sample.window_title);
    let title = if settings.store_window_titles {
        truncate(&sample.window_title, 300)
    } else {
        String::new()
    };
    let idle = sample
        .idle_for
        .map(|d| d.as_secs() >= settings.idle_threshold_secs)
        .unwrap_or(false);
    let app = truncate(&sample.app_name, 120);
    set_live(state, &app, category.as_str(), now);

    if let Some(current) = open {
        if current.app == app
            && current.title == title
            && current.category == category.as_str()
            && current.idle == idle
        {
            state.db.touch_segment(current.id, now)?;
            return Ok(());
        }
    }
    let id = state.db.insert_segment(
        now,
        now,
        &app,
        &title,
        category.as_str(),
        idle,
    )?;
    *open = Some(OpenSeg {
        id,
        app,
        title,
        category: category.as_str().to_string(),
        idle,
    });
    Ok(())
}

fn set_live(state: &AppState, app: &str, category: &str, at: i64) {
    if let Ok(mut slot) = state.live_app.lock() {
        *slot = app.to_string();
    }
    if let Ok(mut slot) = state.live_category.lock() {
        *slot = category.to_string();
    }
    if let Ok(mut slot) = state.live_at.lock() {
        *slot = at;
    }
}

fn is_self(app: &str) -> bool {
    app.to_lowercase().contains("tidskvitto")
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
