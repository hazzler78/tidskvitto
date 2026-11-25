use serde::{Deserialize, Serialize};

/// Tak per kategori så att ett antagande inte kan säga att en agent tar allt.
pub const ASSUMPTION_CAP: f64 = 0.8;
pub const DEFAULT_EMAIL_RATE: f64 = 0.4;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assumptions {
    pub epost: f64,
    pub koordinering: f64,
    pub moten: f64,
    pub fokus: f64,
    pub ovrigt: f64,
}

impl Default for Assumptions {
    fn default() -> Self {
        Self {
            epost: DEFAULT_EMAIL_RATE,
            koordinering: 0.2,
            moten: 0.1,
            fokus: 0.15,
            ovrigt: 0.0,
        }
    }
}

impl Assumptions {
    pub fn clamped(self) -> Self {
        Self {
            epost: clamp_rate(self.epost),
            koordinering: clamp_rate(self.koordinering),
            moten: clamp_rate(self.moten),
            fokus: clamp_rate(self.fokus),
            ovrigt: clamp_rate(self.ovrigt),
        }
    }

    pub fn for_category(self, category: crate::Category) -> f64 {
        match category {
            crate::Category::Epost => self.epost,
            crate::Category::Koordinering => self.koordinering,
            crate::Category::Mote => self.moten,
            crate::Category::Fokus => self.fokus,
            crate::Category::Ovrigt => self.ovrigt,
        }
    }
}

pub fn clamp_rate(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    value.clamp(0.0, ASSUMPTION_CAP)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub tracking_enabled: bool,
    pub onboarded: bool,
    pub idle_threshold_secs: u64,
    pub store_window_titles: bool,
    pub share_titles_with_ai: bool,
    pub ai_provider: String,
    pub ai_base_url: String,
    pub ai_model: String,
    /// Sant när personen själv valt leverantör. Då lämnar vi valet orört.
    #[serde(default)]
    pub ai_choice_saved: bool,
    pub autostart: bool,
    pub assumptions: Assumptions,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // Mätningen startar först när personen sagt ja i integritetsrutan.
            tracking_enabled: false,
            onboarded: false,
            idle_threshold_secs: 180,
            store_window_titles: true,
            share_titles_with_ai: false,
            ai_provider: "grok".to_string(),
            ai_base_url: String::new(),
            ai_model: String::new(),
            ai_choice_saved: false,
            autostart: false,
            assumptions: Assumptions::default(),
        }
    }
}

impl Settings {
    pub fn normalized(mut self) -> Self {
        self.idle_threshold_secs = self.idle_threshold_secs.clamp(30, 1800);
        self.ai_provider = normalize_provider(&self.ai_provider);
        self.ai_base_url = self.ai_base_url.trim().trim_end_matches('/').to_string();
        self.ai_model = sanitize_model(&self.ai_model);
        if !self.ai_choice_saved
            && self.ai_provider == "openai"
            && self.ai_model.is_empty()
            && self.ai_base_url.is_empty()
        {
            self.ai_provider = "grok".to_string();
        }
        self.assumptions = self.assumptions.clamped();
        self.share_titles_with_ai = self.share_titles_with_ai && self.store_window_titles;
        self
    }
}

fn normalize_provider(value: &str) -> String {
    match value.trim() {
        "grok" | "xai" => "grok".to_string(),
        "anthropic" => "anthropic".to_string(),
        "compatible" => "compatible".to_string(),
        _ => "openai".to_string(),
    }
}

fn sanitize_model(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(80)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Basadressen måste vara https, utom loopback under lokal utveckling.
pub fn validate_base_url(url: &str) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Ok(());
    }
    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| "Basadressen måste börja med https://.".to_string())?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.split('@').next_back().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host).to_ascii_lowercase();
    match scheme {
        "https" => {
            if host.is_empty() {
                return Err("Basadressen saknar värdnamn.".into());
            }
            Ok(())
        }
        "http" if host == "localhost" || host == "127.0.0.1" || host == "::1" => Ok(()),
        "http" => Err("Basadressen måste vara https. Http tillåts bara mot localhost.".into()),
        _ => Err("Basadressen måste vara https.".into()),
    }
}

pub fn default_base_url(provider: &str) -> &'static str {
    match provider {
        "grok" => "https://api.x.ai/v1",
        "anthropic" => "https://api.anthropic.com/v1",
        _ => "https://api.openai.com/v1",
    }
}

pub fn default_model(provider: &str) -> &'static str {
    match provider {
        "grok" => "grok-4.7",
        "anthropic" => "claude-sonnet-4-5",
        _ => "gpt-4o-mini",
    }
}

pub fn resolved_base_url(settings: &Settings) -> String {
    if !settings.ai_base_url.is_empty() {
        settings.ai_base_url.clone()
    } else {
        default_base_url(&settings.ai_provider).to_string()
    }
}

pub fn resolved_model(settings: &Settings) -> String {
    if !settings.ai_model.is_empty() {
        settings.ai_model.clone()
    } else {
        default_model(&settings.ai_provider).to_string()
    }
}

pub fn endpoint_host(settings: &Settings) -> String {
    let base = resolved_base_url(settings);
    base.split("://")
        .nth(1)
        .unwrap_or(&base)
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(&base)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates_cannot_exceed_the_cap_or_go_negative() {
        assert_eq!(clamp_rate(-1.0), 0.0);
        assert_eq!(clamp_rate(0.4), 0.4);
        assert_eq!(clamp_rate(1.2), ASSUMPTION_CAP);
        assert_eq!(clamp_rate(f64::NAN), 0.0);
    }

    #[test]
    fn base_url_rejects_plain_http_except_localhost() {
        assert!(validate_base_url("").is_ok());
        assert!(validate_base_url("https://api.openai.com/v1").is_ok());
        assert!(validate_base_url("http://localhost:8080/v1").is_ok());
        assert!(validate_base_url("http://example.com").is_err());
        assert!(validate_base_url("file:///tmp/x").is_err());
    }

    #[test]
    fn fresh_install_uses_grok_without_a_banner() {
        let settings = Settings::default().normalized();
        assert_eq!(settings.ai_provider, "grok");
        assert_eq!(resolved_model(&settings), "grok-4.7");
        assert_eq!(endpoint_host(&settings), "api.x.ai");
    }

    #[test]
    fn untouched_openai_default_moves_to_grok_but_an_explicit_choice_stays() {
        let mut legacy = Settings::default();
        legacy.ai_provider = "openai".to_string();
        legacy.ai_choice_saved = false;
        assert_eq!(legacy.normalized().ai_provider, "grok");

        let mut chosen = Settings::default();
        chosen.ai_provider = "openai".to_string();
        chosen.ai_choice_saved = true;
        let chosen = chosen.normalized();
        assert_eq!(chosen.ai_provider, "openai");
        assert_eq!(resolved_model(&chosen), "gpt-4o-mini");
    }

    #[test]
    fn sharing_titles_requires_storing_them() {
        let mut settings = Settings::default();
        settings.store_window_titles = false;
        settings.share_titles_with_ai = true;
        let settings = settings.normalized();
        assert!(!settings.share_titles_with_ai);
    }
}
