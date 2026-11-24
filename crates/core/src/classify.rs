use serde::{Deserialize, Serialize};

/// Arbetsytor som kvittot summerar. Ordningen är den som visas på kvittot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Fokus,
    Epost,
    Koordinering,
    Mote,
    Ovrigt,
}

impl Category {
    pub fn all() -> [Category; 5] {
        [
            Category::Fokus,
            Category::Epost,
            Category::Koordinering,
            Category::Mote,
            Category::Ovrigt,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Category::Fokus => "Fokusarbete",
            Category::Epost => "E-post",
            Category::Koordinering => "Koordinering",
            Category::Mote => "Möten",
            Category::Ovrigt => "Övrigt",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Category::Fokus => "fokus",
            Category::Epost => "epost",
            Category::Koordinering => "koordinering",
            Category::Mote => "mote",
            Category::Ovrigt => "ovrigt",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "fokus" => Category::Fokus,
            "epost" => Category::Epost,
            "koordinering" => Category::Koordinering,
            "mote" => Category::Mote,
            _ => Category::Ovrigt,
        }
    }
}

/// Klassar aktivt appnamn och fönstertitel. Titeln används bara som text,
/// aldrig som innehåll ur mejlet. Webmejl känns igen på titeln eftersom
/// vi medvetet inte läser webbläsarens adress.
pub fn classify(app_name: &str, window_title: &str) -> Category {
    let app = norm_app(app_name);
    let title = window_title.to_lowercase();
    if app.is_empty() {
        return Category::Ovrigt;
    }

    if is_meeting_app(&app) {
        return Category::Mote;
    }

    if is_email_app(&app) {
        if title_has_live_meeting(&title) {
            return Category::Mote;
        }
        return Category::Epost;
    }

    if is_teams(&app) || is_slack(&app) || is_discord(&app) || is_skype(&app) {
        if title_has_live_meeting(&title) || title.contains("huddle") {
            return Category::Mote;
        }
        return Category::Koordinering;
    }

    if is_browser(&app) {
        if title_is_webmail(&title) {
            return Category::Epost;
        }
        if title_is_webmeeting(&title) {
            return Category::Mote;
        }
        return Category::Fokus;
    }

    if is_calendar(&app) || is_chat_app(&app) {
        return Category::Koordinering;
    }

    if is_focus_app(&app) {
        return Category::Fokus;
    }

    Category::Ovrigt
}

fn norm_app(app_name: &str) -> String {
    let lower = app_name.trim().to_lowercase();
    lower
        .strip_suffix(".exe")
        .unwrap_or(&lower)
        .trim()
        .to_string()
}

fn is_email_app(app: &str) -> bool {
    const NAMED: &[&str] = &[
        "outlook",
        "olk",
        "hxoutlook",
        "thunderbird",
        "mailspring",
        "mimestream",
        "airmail",
        "superhuman",
        "postbox",
        "emclient",
        "em client",
        "canarymail",
        "canary mail",
        "spark desktop",
        "spark",
    ];
    if NAMED.iter().any(|n| app == *n || app.contains(n)) {
        // "spark" is a short name; still a known mail client on the desktop.
        return true;
    }
    app == "mail" || app == "apple mail" || app.ends_with(" mail")
}

fn is_meeting_app(app: &str) -> bool {
    const NAMED: &[&str] = &[
        "zoom",
        "webex",
        "facetime",
        "gotomeeting",
        "bluejeans",
        "google meet",
    ];
    NAMED.iter().any(|n| app == *n || app.contains(n))
}

fn is_teams(app: &str) -> bool {
    app == "teams" || app.contains("microsoft teams") || app.ends_with(" teams")
}

fn is_slack(app: &str) -> bool {
    app == "slack" || app.contains("slack")
}

fn is_discord(app: &str) -> bool {
    app.contains("discord")
}

fn is_skype(app: &str) -> bool {
    app.contains("skype")
}

fn is_browser(app: &str) -> bool {
    const EXACT: &[&str] = &["arc", "dia", "orion", "safari", "chrome", "edge", "opera"];
    if EXACT.contains(&app) {
        return true;
    }
    const PARTS: &[&str] = &[
        "google chrome",
        "firefox",
        "microsoft edge",
        "msedge",
        "brave",
        "vivaldi",
        "chromium",
        "safari",
    ];
    PARTS.iter().any(|p| app.contains(p))
}

fn is_calendar(app: &str) -> bool {
    const NAMED: &[&str] = &["calendar", "kalender", "fantastical", "busycal"];
    NAMED.iter().any(|n| app == *n || app.contains(n))
}

fn is_chat_app(app: &str) -> bool {
    const NAMED: &[&str] = &[
        "telegram",
        "whatsapp",
        "messenger",
        "signal",
        "messages",
        "meddelanden",
        "imessage",
    ];
    NAMED.iter().any(|n| app == *n || app.contains(n))
}

fn is_focus_app(app: &str) -> bool {
    const PARTS: &[&str] = &[
        "visual studio",
        "vscode",
        "cursor",
        "xcode",
        "android studio",
        "webstorm",
        "pycharm",
        "goland",
        "intellij",
        "rider",
        "terminal",
        "iterm",
        "warp",
        "alacritty",
        "wezterm",
        "windowsterminal",
        "powershell",
        "word",
        "excel",
        "powerpoint",
        "libreoffice",
        "pages",
        "numbers",
        "keynote",
        "figma",
        "sketch",
        "photoshop",
        "illustrator",
        "affinity",
        "notion",
        "obsidian",
        "onenote",
        "typora",
        "logseq",
        "linear",
        "jira",
        "anteckningar",
    ];
    if app == "code" || app == "notes" || app == "cmd" || app == "wt" {
        return true;
    }
    PARTS.iter().any(|p| app.contains(p))
}

fn title_has_live_meeting(title: &str) -> bool {
    const MARKERS: &[&str] = &[
        "möte",
        "mote",
        "meeting",
        "huddle",
        "samtal",
        "google meet",
        "zoom meeting",
        "webex",
    ];
    MARKERS.iter().any(|m| title.contains(m))
}

fn title_is_webmail(title: &str) -> bool {
    const MARKERS: &[&str] = &[
        "gmail",
        "inkorg",
        "mail.google",
        "yahoo mail",
        "proton mail",
        "protonmail",
        "outlook",
        "fastmail",
        "webmail",
        "hotmail",
        "roundcube",
        "zimbra",
    ];
    MARKERS.iter().any(|m| title.contains(m))
}

fn title_is_webmeeting(title: &str) -> bool {
    const MARKERS: &[&str] = &[
        "google meet",
        "meet.google",
        "zoom meeting",
        "zoom-",
        "webex",
        "teams-möte",
        "teams meeting",
    ];
    MARKERS.iter().any(|m| title.contains(m))
}

/// Förenklar en fönstertitel så att "Re: Offert" och "Sv: Offert - Gmail"
/// räknas som samma ärende. Används bara lokalt.
pub fn normalize_title(title: &str) -> String {
    let mut s = title.trim().to_lowercase();
    let suffixes = [
        " - google chrome",
        " - chrome",
        " - firefox",
        " - mozilla firefox",
        " - safari",
        " - microsoft edge",
        " - edge",
        " - brave",
        " - arc",
        " - gmail",
        " - outlook",
        " - mail",
    ];
    loop {
        let before = s.clone();
        for suffix in suffixes {
            if let Some(stripped) = s.strip_suffix(suffix) {
                s = stripped.trim().to_string();
            }
        }
        if s == before {
            break;
        }
    }
    let prefixes = ["re:", "sv:", "fwd:", "fw:", "vb:", "aw:", "vs:", "ang:"];
    loop {
        let before = s.clone();
        let trimmed = s.trim().to_string();
        for prefix in prefixes {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                s = rest.trim().to_string();
            }
        }
        if s == before {
            break;
        }
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn is_generic_title(normalized: &str) -> bool {
    if normalized.chars().count() < 4 {
        return true;
    }
    matches!(
        normalized,
        "gmail"
            | "inkorg"
            | "inbox"
            | "outlook"
            | "mail"
            | "e-post"
            | "epost"
            | "nytt meddelande"
            | "new message"
            | "new mail"
            | "post"
            | "google chrome"
            | "chrome"
            | "safari"
            | "firefox"
            | "microsoft edge"
            | "edge"
            | "hotmail"
            | "webmail"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(app: &str, title: &str) -> Category {
        classify(app, title)
    }

    #[test]
    fn email_clients_and_webmail() {
        assert_eq!(c("Microsoft Outlook", "Inkorg"), Category::Epost);
        assert_eq!(c("olk.exe", "Inbox"), Category::Epost);
        assert_eq!(c("Mail", "Hej"), Category::Epost);
        assert_eq!(c("Thunderbird", ""), Category::Epost);
        assert_eq!(
            c("Google Chrome", "Inkorg (2) - a@b.se - Gmail"),
            Category::Epost
        );
        assert_eq!(c("Safari", "Proton Mail"), Category::Epost);
        assert_eq!(c("Firefox", "Mail - Sara - Outlook"), Category::Epost);
    }

    #[test]
    fn meetings_are_not_confused_with_notes() {
        assert_eq!(c("zoom.us", "Standup"), Category::Mote);
        assert_eq!(c("Microsoft Teams", "Veckomöte"), Category::Mote);
        assert_eq!(c("Microsoft Teams", "Chatt | Marknad"), Category::Koordinering);
        assert_eq!(c("Slack", "allmänt"), Category::Koordinering);
        assert_eq!(c("Slack", "Huddle with Anna"), Category::Mote);
        assert_eq!(c("Google Chrome", "Standup - Google Meet"), Category::Mote);
        assert_eq!(c("Microsoft Word", "Mötesanteckningar"), Category::Fokus);
        assert_eq!(
            c("Microsoft Outlook", "Budgetmöte"),
            Category::Mote
        );
    }

    #[test]
    fn browsers_default_to_focus_and_music_is_other() {
        assert_eq!(
            c("Google Chrome", "Tidskvitto - Google Docs"),
            Category::Fokus
        );
        assert_eq!(c("Cursor", "README.md"), Category::Fokus);
        assert_eq!(c("Code.exe", "main.rs"), Category::Fokus);
        assert_eq!(c("Spotify", "Spellista"), Category::Ovrigt);
        assert_eq!(c("Finder", ""), Category::Ovrigt);
        assert_eq!(c("Kalender", "November"), Category::Koordinering);
    }

    #[test]
    fn normalize_reply_prefixes_and_browser_suffix() {
        assert_eq!(
            normalize_title("Sv: Re: Offert Q4 - a@b.se - Gmail"),
            "offert q4 - a@b.se"
        );
        assert!(is_generic_title("inkorg"));
        assert!(!is_generic_title("offert q4"));
    }
}
