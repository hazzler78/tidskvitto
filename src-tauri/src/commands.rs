use std::sync::Arc;

use chrono::{Duration, Local};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tidskvitto_core::{
    ai_payload, build_insights, build_receipt, endpoint_host, plan_question, render_local_answer,
    resolved_model, span_for, validate_base_url, AiTitles, Assumptions, RangeKind,
};

use crate::platform;
use crate::secrets;
use crate::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicSettings {
    pub tracking_enabled: bool,
    pub onboarded: bool,
    pub idle_threshold_secs: u64,
    pub store_window_titles: bool,
    pub share_titles_with_ai: bool,
    pub ai_provider: String,
    pub ai_base_url: String,
    pub ai_model: String,
    pub autostart: bool,
    pub assumptions: Assumptions,
    pub has_api_key: bool,
    pub key_storage: String,
    pub endpoint_host: String,
    pub resolved_model: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostStatus {
    pub os: String,
    pub screen_recording: String,
    pub idle_detection: String,
    pub data_dir: String,
    pub tracking_enabled: bool,
    pub onboarded: bool,
    pub last_app: String,
    pub last_category: String,
    pub last_sample_at: i64,
    pub has_api_key: bool,
    pub endpoint_host: String,
    pub share_titles_with_ai: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub tracking_enabled: Option<bool>,
    pub onboarded: Option<bool>,
    pub idle_threshold_secs: Option<u64>,
    pub store_window_titles: Option<bool>,
    pub share_titles_with_ai: Option<bool>,
    pub ai_provider: Option<String>,
    pub ai_base_url: Option<String>,
    pub ai_model: Option<String>,
    pub autostart: Option<bool>,
    pub assumptions: Option<Assumptions>,
    pub api_key: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatReply {
    pub text: String,
    pub source: String,
    pub used_titles: bool,
    pub period_label: String,
}

#[tauri::command]
pub fn status(state: State<'_, Arc<AppState>>) -> HostStatus {
    let settings = state.db.load_settings();
    let key = secrets::load_api_key(&state.db);
    HostStatus {
        os: std::env::consts::OS.to_string(),
        screen_recording: platform::screen_recording().to_string(),
        idle_detection: if state.idle_supported.load(std::sync::atomic::Ordering::Relaxed) {
            "available".into()
        } else {
            "unavailable".into()
        },
        data_dir: state.db.path().display().to_string(),
        tracking_enabled: settings.tracking_enabled,
        onboarded: settings.onboarded,
        last_app: state.live_app.lock().map(|v| v.clone()).unwrap_or_default(),
        last_category: state
            .live_category
            .lock()
            .map(|v| v.clone())
            .unwrap_or_default(),
        last_sample_at: state.live_at.lock().map(|v| *v).unwrap_or(0),
        has_api_key: key.value.is_some(),
        endpoint_host: endpoint_host(&settings),
        share_titles_with_ai: settings.share_titles_with_ai,
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> PublicSettings {
    public_settings(&state)
}

#[tauri::command]
pub fn receipt(
    state: State<'_, Arc<AppState>>,
    range: String,
    anchor: String,
) -> Result<tidskvitto_core::Receipt, String> {
    let settings = state.db.load_settings();
    let today = Local::now().date_naive();
    let anchor = tidskvitto_core::parse_anchor(&anchor, today);
    let kind = RangeKind::parse(&range);
    let span = span_for(kind, anchor, &Local);
    let segments = state.db.segments_overlapping(span.start_ms, span.end_ms)?;
    Ok(build_receipt(&segments, &span, &settings.assumptions))
}

#[tauri::command]
pub fn insights(state: State<'_, Arc<AppState>>) -> Result<tidskvitto_core::Insights, String> {
    let settings = state.db.load_settings();
    let today = Local::now().date_naive();
    let first = tidskvitto_core::monday_of(today) - chrono::Duration::days(5 * 7);
    let start = span_for(RangeKind::Week, first, &Local);
    let end = span_for(RangeKind::Week, today, &Local);
    let segments = state
        .db
        .segments_overlapping(start.start_ms, end.end_ms)?;
    Ok(build_insights(
        &segments,
        today,
        &Local,
        &settings.assumptions,
    ))
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    patch: SettingsPatch,
) -> Result<PublicSettings, String> {
    let mut settings = state.db.load_settings();
    if let Some(value) = patch.tracking_enabled {
        settings.tracking_enabled = value;
    }
    if let Some(value) = patch.onboarded {
        settings.onboarded = value;
    }
    if let Some(value) = patch.idle_threshold_secs {
        settings.idle_threshold_secs = value;
    }
    if let Some(value) = patch.store_window_titles {
        settings.store_window_titles = value;
    }
    if let Some(value) = patch.share_titles_with_ai {
        settings.share_titles_with_ai = value;
    }
    let chose_provider = patch.ai_provider.is_some()
        || patch.ai_model.is_some()
        || patch.ai_base_url.is_some();
    if let Some(value) = patch.ai_provider {
        settings.ai_provider = value;
    }
    if let Some(value) = patch.ai_base_url {
        validate_base_url(&value)?;
        settings.ai_base_url = value;
    }
    if let Some(value) = patch.ai_model {
        settings.ai_model = value;
    }
    if chose_provider {
        settings.ai_choice_saved = true;
    }
    if let Some(value) = patch.autostart {
        settings.autostart = value;
    }
    if let Some(value) = patch.assumptions {
        settings.assumptions = value;
    }
    settings = settings.normalized();
    validate_base_url(&settings.ai_base_url)?;
    if settings.ai_provider == "compatible" && settings.ai_base_url.is_empty() {
        return Err("Ange en https-adress för den egna leverantören.".into());
    }
    state.db.save_settings(&settings)?;
    if let Some(key) = patch.api_key {
        secrets::store_api_key(&state.db, &key)?;
    }
    if let Some(enabled) = patch.autostart {
        apply_autostart(&app, enabled)?;
    }
    Ok(public_settings(&state))
}

#[tauri::command]
pub fn delete_measurements(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.db.delete_measurements()?;
    Ok(())
}

#[tauri::command]
pub fn delete_everything(state: State<'_, Arc<AppState>>) -> Result<PublicSettings, String> {
    secrets::clear_api_key(&state.db)?;
    state.db.reset_all()?;
    Ok(public_settings(&state))
}

#[tauri::command]
pub fn request_screen_permission() -> Result<String, String> {
    platform::request_screen_recording().map(|v| v.to_string())
}

#[tauri::command]
pub async fn ask(state: State<'_, Arc<AppState>>, question: String) -> Result<ChatReply, String> {
    let question = question.trim().to_string();
    if question.is_empty() {
        return Err("Skriv en fråga.".into());
    }
    if question.chars().count() > 2000 {
        return Err("Frågan är för lång.".into());
    }
    let settings = state.db.load_settings();
    let now = Local::now();
    let plan = plan_question(&question, now);
    let span = span_for(plan.kind, plan.anchor, &Local);
    let segments = state.db.segments_overlapping(span.start_ms, span.end_ms)?;
    let receipt = build_receipt(&segments, &span, &settings.assumptions);
    let local = render_local_answer(&plan, &receipt);
    let key = secrets::load_api_key(&state.db);
    let Some(api_key) = key.value else {
        return Ok(ChatReply {
            text: local,
            source: "lokal".into(),
            used_titles: false,
            period_label: receipt.period_label,
        });
    };

    let today = now.date_naive();
    let mut recent = Vec::new();
    for offset in (0..14).rev() {
        let date = today - Duration::days(offset);
        let day = span_for(RangeKind::Day, date, &Local);
        let day_segments = state
            .db
            .segments_overlapping(day.start_ms, day.end_ms)?;
        let day_receipt = build_receipt(&day_segments, &day, &settings.assumptions);
        if let Some(bucket) = day_receipt.days.into_iter().next() {
            recent.push(bucket);
        }
    }
    let used_titles = settings.share_titles_with_ai;
    let titles = if used_titles {
        tidskvitto_core::top_titles(&segments, span.start_ms, span.end_ms, 15)
    } else {
        Vec::new()
    };
    let payload = if used_titles {
        ai_payload(&receipt, &recent, AiTitles::Included(&titles))
    } else {
        ai_payload(&receipt, &recent, AiTitles::Omitted)
    };
    match crate::ai::complete(&settings, &api_key, &payload, &question).await {
        Ok(text) => Ok(ChatReply {
            text,
            source: "ai".into(),
            used_titles,
            period_label: receipt.period_label,
        }),
        Err(err) => Ok(ChatReply {
            text: format!(
                "{err} Här är ett lokalt svar i stället, utifrån samma sammanfattning.\n\n{local}"
            ),
            source: "lokal".into(),
            used_titles: false,
            period_label: receipt.period_label,
        }),
    }
}

fn public_settings(state: &AppState) -> PublicSettings {
    let settings = state.db.load_settings();
    let key = secrets::load_api_key(&state.db);
    let endpoint = endpoint_host(&settings);
    let model = resolved_model(&settings);
    PublicSettings {
        tracking_enabled: settings.tracking_enabled,
        onboarded: settings.onboarded,
        idle_threshold_secs: settings.idle_threshold_secs,
        store_window_titles: settings.store_window_titles,
        share_titles_with_ai: settings.share_titles_with_ai,
        ai_provider: settings.ai_provider,
        ai_base_url: settings.ai_base_url,
        ai_model: settings.ai_model,
        autostart: settings.autostart,
        assumptions: settings.assumptions,
        has_api_key: key.value.is_some(),
        key_storage: key.storage,
        endpoint_host: endpoint,
        resolved_model: model,
    }
}

fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autolaunch();
    if enabled {
        autostart
            .enable()
            .map_err(|e| format!("Kunde inte slå på start vid inloggning: {e}"))?;
    } else {
        autostart
            .disable()
            .map_err(|e| format!("Kunde inte stänga av start vid inloggning: {e}"))?;
    }
    Ok(())
}
