use std::collections::HashMap;

use serde::Serialize;
use serde_json::{json, Value};

use crate::classify::{is_generic_title, normalize_title, Category};
use crate::{estimate_basis, estimate_note};
use crate::settings::Assumptions;
use crate::timeutil::Span;
use crate::Segment;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTotal {
    pub category: Category,
    pub label: String,
    pub active_ms: u64,
    pub share: f64,
    pub assumption: f64,
    pub estimated_savings_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppTotal {
    pub app_name: String,
    pub category: Category,
    pub active_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayBucket {
    pub date: String,
    pub label: String,
    pub active_ms: u64,
    pub email_ms: u64,
    pub estimated_savings_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub id: String,
    pub title: String,
    pub body: String,
    pub basis: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleStat {
    pub title: String,
    pub visits: u32,
    pub active_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    pub range: String,
    pub anchor_date: String,
    pub period_label: String,
    pub timezone: String,
    pub active_ms: u64,
    pub idle_ms: u64,
    pub email_ms: u64,
    pub email_share: f64,
    pub coordination_ms: u64,
    pub coordination_share: f64,
    pub meeting_ms: u64,
    pub categories: Vec<CategoryTotal>,
    pub apps: Vec<AppTotal>,
    pub days: Vec<DayBucket>,
    pub estimated_savings_ms: u64,
    pub assumptions: Assumptions,
    pub estimate_note: String,
    pub estimate_basis: String,
    pub suggestions: Vec<Suggestion>,
    pub plaintext: String,
}

pub fn format_duration_sv(ms: u64) -> String {
    let total_min = (ms + 30_000) / 60_000;
    if total_min < 60 {
        return format!("{total_min} min");
    }
    let hours = total_min / 60;
    let minutes = total_min % 60;
    if minutes == 0 {
        format!("{hours} tim")
    } else {
        format!("{hours} tim {minutes} min")
    }
}

pub fn format_percent_sv(share: f64) -> String {
    let pct = if share.is_finite() {
        (share * 100.0).round().clamp(0.0, 100.0) as i64
    } else {
        0
    };
    format!("{pct}\u{00a0}%")
}

pub fn savings_ms(active_ms: u64, rate: f64) -> u64 {
    let rate = crate::clamp_rate(rate);
    ((active_ms as f64) * rate).round() as u64
}

pub fn build_receipt(segments: &[Segment], span: &Span, assumptions: &Assumptions) -> Receipt {
    let assumptions = assumptions.clamped();
    let mut by_cat: HashMap<Category, u64> = HashMap::new();
    let mut by_app: HashMap<(String, Category), u64> = HashMap::new();
    let mut idle_ms = 0u64;
    let mut titles: HashMap<String, TitleAcc> = HashMap::new();

    for segment in segments {
        let ms = overlap_ms(segment.started_at, segment.ended_at, span.start_ms, span.end_ms);
        if ms == 0 {
            continue;
        }
        if segment.idle {
            idle_ms += ms;
            continue;
        }
        let category = Category::parse(&segment.category);
        *by_cat.entry(category).or_insert(0) += ms;
        let app = if segment.app_name.trim().is_empty() {
            "Okänd app".to_string()
        } else {
            segment.app_name.trim().to_string()
        };
        *by_app.entry((app, category)).or_insert(0) += ms;
        if category == Category::Epost {
            accumulate_title(&mut titles, &segment.window_title, ms);
        }
    }

    let active_ms: u64 = by_cat.values().sum();
    let categories = Category::all()
        .into_iter()
        .map(|category| {
            let ms = by_cat.get(&category).copied().unwrap_or(0);
            let rate = assumptions.for_category(category);
            CategoryTotal {
                category,
                label: category.label().to_string(),
                active_ms: ms,
                share: share(ms, active_ms),
                assumption: rate,
                estimated_savings_ms: savings_ms(ms, rate),
            }
        })
        .collect::<Vec<_>>();
    let estimated_savings_ms = categories.iter().map(|c| c.estimated_savings_ms).sum();
    let email_ms = by_cat.get(&Category::Epost).copied().unwrap_or(0);
    let coordination_ms = by_cat.get(&Category::Koordinering).copied().unwrap_or(0);
    let meeting_ms = by_cat.get(&Category::Mote).copied().unwrap_or(0);

    let mut apps = by_app
        .into_iter()
        .map(|((app_name, category), active_ms)| AppTotal {
            app_name,
            category,
            active_ms,
        })
        .collect::<Vec<_>>();
    apps.sort_by(|a, b| b.active_ms.cmp(&a.active_ms).then(a.app_name.cmp(&b.app_name)));
    apps.truncate(8);

    let days = span
        .days
        .iter()
        .map(|day| {
            let mut active = 0u64;
            let mut email = 0u64;
            let mut saved = 0u64;
            let mut cat_ms: HashMap<Category, u64> = HashMap::new();
            for segment in segments {
                let ms = overlap_ms(segment.started_at, segment.ended_at, day.start_ms, day.end_ms);
                if ms == 0 || segment.idle {
                    continue;
                }
                let category = Category::parse(&segment.category);
                active += ms;
                *cat_ms.entry(category).or_insert(0) += ms;
                if category == Category::Epost {
                    email += ms;
                }
            }
            for (category, ms) in cat_ms {
                saved += savings_ms(ms, assumptions.for_category(category));
            }
            DayBucket {
                date: day.date.format("%Y-%m-%d").to_string(),
                label: day.label.clone(),
                active_ms: active,
                email_ms: email,
                estimated_savings_ms: saved,
            }
        })
        .collect::<Vec<_>>();

    let repeats = title_stats(&titles);
    let suggestions = build_suggestions(email_ms, coordination_ms, active_ms, estimated_savings_ms, &repeats);

    let mut receipt = Receipt {
        range: span.kind.as_str().to_string(),
        anchor_date: span.anchor_date.clone(),
        period_label: span.period_label.clone(),
        timezone: span.timezone.clone(),
        active_ms,
        idle_ms,
        email_ms,
        email_share: share(email_ms, active_ms),
        coordination_ms,
        coordination_share: share(coordination_ms, active_ms),
        meeting_ms,
        categories,
        apps,
        days,
        estimated_savings_ms,
        assumptions,
        estimate_note: estimate_note().to_string(),
        estimate_basis: estimate_basis().to_string(),
        suggestions,
        plaintext: String::new(),
    };
    receipt.plaintext = plaintext(&receipt);
    receipt
}

pub fn top_titles(segments: &[Segment], start_ms: i64, end_ms: i64, limit: usize) -> Vec<TitleStat> {
    let mut titles: HashMap<String, TitleAcc> = HashMap::new();
    for segment in segments {
        if segment.idle || Category::parse(&segment.category) != Category::Epost {
            continue;
        }
        let ms = overlap_ms(segment.started_at, segment.ended_at, start_ms, end_ms);
        if ms == 0 {
            continue;
        }
        accumulate_title(&mut titles, &segment.window_title, ms);
    }
    let mut stats = title_stats(&titles);
    stats.truncate(limit);
    stats
}

fn plaintext(receipt: &Receipt) -> String {
    let mut lines = vec![
        "TIDSKVITTO".to_string(),
        "4days.ai".to_string(),
        receipt.period_label.clone(),
        String::new(),
        format!("Aktiv tid: {}", format_duration_sv(receipt.active_ms)),
        format!(
            "Tomgång (räknas inte): {}",
            format_duration_sv(receipt.idle_ms)
        ),
        String::new(),
    ];
    for category in &receipt.categories {
        lines.push(format!(
            "{}: {}",
            category.label,
            format_duration_sv(category.active_ms)
        ));
    }
    lines.push(String::new());
    lines.push(format!(
        "E-postandel: {}",
        format_percent_sv(receipt.email_share)
    ));
    lines.push(format!(
        "Koordinering: {}",
        format_percent_sv(receipt.coordination_share)
    ));
    lines.push(String::new());
    lines.push(format!(
        "Uppskattad tid en agent skulle kunna ta: {}",
        format_duration_sv(receipt.estimated_savings_ms)
    ));
    lines.push("Det här är en uppskattning, inte en mätning.".to_string());
    lines.push(receipt.estimate_note.clone());
    lines.push(receipt.estimate_basis.clone());
    lines.push(String::new());
    lines.push(crate::TAGLINE.to_string());
    lines.join("\n")
}

struct TitleAcc {
    display: String,
    visits: u32,
    active_ms: u64,
}

fn accumulate_title(map: &mut HashMap<String, TitleAcc>, raw: &str, ms: u64) {
    let normalized = normalize_title(raw);
    if is_generic_title(&normalized) {
        return;
    }
    let entry = map.entry(normalized).or_insert_with(|| TitleAcc {
        display: redact_title(raw),
        visits: 0,
        active_ms: 0,
    });
    entry.visits += 1;
    entry.active_ms += ms;
}

fn title_stats(map: &HashMap<String, TitleAcc>) -> Vec<TitleStat> {
    let mut stats = map
        .values()
        .map(|acc| TitleStat {
            title: truncate_chars(&acc.display, 80),
            visits: acc.visits,
            active_ms: acc.active_ms,
        })
        .collect::<Vec<_>>();
    stats.sort_by(|a, b| b.active_ms.cmp(&a.active_ms).then(b.visits.cmp(&a.visits)));
    stats
}

fn build_suggestions(
    email_ms: u64,
    coordination_ms: u64,
    active_ms: u64,
    estimated_ms: u64,
    repeats: &[TitleStat],
) -> Vec<Suggestion> {
    let mut out = Vec::new();
    for (index, title) in repeats.iter().enumerate() {
        if title.visits < 3 || title.active_ms < 8 * 60_000 {
            continue;
        }
        out.push(Suggestion {
            id: format!("repeat-{index}"),
            title: "Återkommande e-postfönster".to_string(),
            body: format!(
                "Fönstret \"{}\" har varit aktivt {} gånger ({}). Det kan vara ett återkommande ärende en agent kan förbereda svar till.",
                title.title,
                title.visits,
                format_duration_sv(title.active_ms)
            ),
            basis: format!(
                "Grund: {} besök i e-post med liknande fönstertitel, sammanlagt {}. Tidskvitto har inte läst mejlet och skickar inget.",
                title.visits,
                format_duration_sv(title.active_ms)
            ),
        });
        if out.len() == 2 {
            break;
        }
    }
    if email_ms >= 45 * 60_000 && share(email_ms, active_ms) >= 0.2 {
        out.push(Suggestion {
            id: "email-share".to_string(),
            title: "E-post tar en stor del av tiden".to_string(),
            body: format!(
                "E-post är {} av den aktiva tiden ({}). Med standardantagandet 40 % skulle en agent kunna ta ungefär {} av den tiden — sortering och utkast, inte beslut.",
                format_percent_sv(share(email_ms, active_ms)),
                format_duration_sv(email_ms),
                format_duration_sv(savings_ms(email_ms, 0.4))
            ),
            basis: "Grund: uppmätt tid i mejlprogram och webbmejl, gånger antagandet 40 %. Ingen mejltext har lästs.".to_string(),
        });
    }
    if coordination_ms >= 40 * 60_000 {
        out.push(Suggestion {
            id: "coordination".to_string(),
            title: "Koordinering går att sammanfatta".to_string(),
            body: format!(
                "Koordinering (chatt och kalender) är {}. En agent kan sammanfatta trådar och föreslå svar, ungefär {} med standardantagandet 20 %.",
                format_duration_sv(coordination_ms),
                format_duration_sv(savings_ms(coordination_ms, 0.2))
            ),
            basis: "Grund: tid i chatt- och kalenderappar, gånger antagandet 20 %.".to_string(),
        });
    }
    if estimated_ms >= 60 * 60_000 {
        out.push(Suggestion {
            id: "savings".to_string(),
            title: "Uppskattningen är värd att titta på".to_string(),
            body: format!(
                "Om antagandena stämmer skulle en agent kunna ta ungefär {} av den här perioden. Det är inte tid du redan sparat.",
                format_duration_sv(estimated_ms)
            ),
            basis: "Grund: summan av kategori gånger antagande. Se formeln på kvittot.".to_string(),
        });
    }
    out.truncate(3);
    out
}

fn share(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        part as f64 / total as f64
    }
}

fn overlap_ms(started: i64, ended: i64, window_start: i64, window_end: i64) -> u64 {
    let start = started.max(window_start);
    let end = ended.min(window_end);
    if end <= start {
        0
    } else {
        (end - start) as u64
    }
}

pub fn redact_title(title: &str) -> String {
    let mut out = String::new();
    for token in title.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        if looks_like_email(token) {
            out.push_str("[mejl]");
        } else {
            out.push_str(token);
        }
    }
    truncate_chars(&out, 80)
}

fn looks_like_email(token: &str) -> bool {
    let token = token.trim_matches(|c: char| matches!(c, ',' | ';' | '(' | ')' | '<' | '>'));
    let Some((name, domain)) = token.split_once('@') else {
        return false;
    };
    !name.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

fn truncate_chars(value: &str, max: usize) -> String {
    let mut out = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index >= max {
            out.push('…');
            break;
        }
        if ch == '\n' || ch == '\r' {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}

pub enum AiTitles<'a> {
    Omitted,
    Included(&'a [TitleStat]),
}

pub fn ai_payload(focused: &Receipt, recent_days: &[DayBucket], titles: AiTitles<'_>) -> Value {
    let categories = focused
        .categories
        .iter()
        .map(|c| {
            json!({
                "namn": c.label,
                "tid": format_duration_sv(c.active_ms),
                "andel": format_percent_sv(c.share),
                "antagande": format_percent_sv(c.assumption),
                "uppskattad_besparing": format_duration_sv(c.estimated_savings_ms),
            })
        })
        .collect::<Vec<_>>();
    let apps = focused
        .apps
        .iter()
        .map(|a| {
            json!({
                "namn": a.app_name,
                "kategori": a.category.label(),
                "tid": format_duration_sv(a.active_ms),
            })
        })
        .collect::<Vec<_>>();
    let recent = recent_days
        .iter()
        .map(|d| {
            json!({
                "datum": d.date,
                "etikett": d.label,
                "aktiv_tid": format_duration_sv(d.active_ms),
                "e_post": format_duration_sv(d.email_ms),
            })
        })
        .collect::<Vec<_>>();
    let title_value = match titles {
        AiTitles::Omitted => json!(
            "Ingår inte. Användaren har inte valt att dela fönstertitlar. Gissa inte ämnen."
        ),
        AiTitles::Included(list) => json!(list
            .iter()
            .map(|t| json!({
                "titel": t.title,
                "besok": t.visits,
                "tid": format_duration_sv(t.active_ms),
            }))
            .collect::<Vec<_>>()),
    };
    json!({
        "kalla": "Tidskvitto, lokalt aggregerad sammanfattning",
        "period": focused.period_label,
        "tidszon": focused.timezone,
        "aktiv_tid": format_duration_sv(focused.active_ms),
        "tomgang_raknas_inte": format_duration_sv(focused.idle_ms),
        "e_post_andel": format_percent_sv(focused.email_share),
        "koordinering_andel": format_percent_sv(focused.coordination_share),
        "uppskattad_besparing": format_duration_sv(focused.estimated_savings_ms),
        "uppskattning": focused.estimate_note,
        "underlag_for_standardantagande": focused.estimate_basis,
        "kategorier": categories,
        "appar": apps,
        "dagar_i_perioden": focused.days.iter().map(|d| json!({
            "datum": d.date,
            "etikett": d.label,
            "aktiv_tid": format_duration_sv(d.active_ms),
            "e_post": format_duration_sv(d.email_ms),
        })).collect::<Vec<_>>(),
        "senaste_14_dagarna": recent,
        "fonster_titlar": title_value,
        "saknas": "Inget mejlinnehåll, inga tangenttryckningar, inga filer och inga webbadresser."
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeutil::{span_for, RangeKind};
    use chrono::{FixedOffset, NaiveDate};

    fn span() -> Span {
        let tz = FixedOffset::east_opt(3600).unwrap();
        span_for(
            RangeKind::Day,
            NaiveDate::from_ymd_opt(2026, 11, 24).unwrap(),
            &tz,
        )
    }

    fn seg(start_min: i64, minutes: i64, app: &str, title: &str, category: &str) -> Segment {
        let span = span();
        let start = span.start_ms + start_min * 60_000;
        Segment {
            id: start,
            started_at: start,
            ended_at: start + minutes * 60_000,
            app_name: app.into(),
            window_title: title.into(),
            category: category.into(),
            idle: false,
        }
    }

    #[test]
    fn estimate_is_the_sum_of_category_rates() {
        let segments = vec![
            seg(8 * 60, 100, "Outlook", "Hej", "epost"),
            seg(10 * 60, 50, "Slack", "", "koordinering"),
        ];
        let receipt = build_receipt(&segments, &span(), &Assumptions::default());
        assert_eq!(receipt.email_ms, 100 * 60_000);
        assert_eq!(receipt.coordination_ms, 50 * 60_000);
        // 40% of 100 min = 40, 20% of 50 min = 10.
        assert_eq!(receipt.estimated_savings_ms, 50 * 60_000);
        assert!(receipt.plaintext.contains("uppskattning"));
        assert!(receipt.plaintext.contains("inte en mätning"));
    }

    #[test]
    fn idle_time_is_excluded_from_active_and_savings() {
        let span = span();
        let mut idle = seg(8 * 60, 30, "Outlook", "Hej", "epost");
        idle.idle = true;
        let receipt = build_receipt(&[idle], &span, &Assumptions::default());
        assert_eq!(receipt.active_ms, 0);
        assert_eq!(receipt.idle_ms, 30 * 60_000);
        assert_eq!(receipt.estimated_savings_ms, 0);
    }

    #[test]
    fn repeated_subjects_become_a_suggestion_generic_inbox_does_not() {
        let mut segments = Vec::new();
        for i in 0..4 {
            segments.push(seg(
                8 * 60 + i * 20,
                10,
                "Outlook",
                "Re: Offert Q4 - a@b.se - Gmail",
                "epost",
            ));
        }
        for i in 0..5 {
            segments.push(seg(14 * 60 + i, 5, "Outlook", "Inkorg - Gmail", "epost"));
        }
        let receipt = build_receipt(&segments, &span(), &Assumptions::default());
        assert!(receipt.suggestions.iter().any(|s| s.id.starts_with("repeat-")));
        let text = receipt
            .suggestions
            .iter()
            .map(|s| s.body.clone())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("[mejl]"));
        assert!(!text.to_lowercase().contains("inkorg"));
    }

    #[test]
    fn ai_payload_omits_titles_unless_included() {
        let segments = vec![seg(9 * 60, 15, "Outlook", "Hemligt ämne", "epost")];
        let receipt = build_receipt(&segments, &span(), &Assumptions::default());
        let hidden = ai_payload(&receipt, &receipt.days, AiTitles::Omitted).to_string();
        assert!(!hidden.contains("Hemligt"));
        assert!(hidden.contains("Ingår inte"));
        let titles = top_titles(&segments, span().start_ms, span().end_ms, 10);
        let shown = ai_payload(&receipt, &receipt.days, AiTitles::Included(&titles)).to_string();
        assert!(shown.contains("Hemligt"));
    }

    #[test]
    fn duration_formatting_is_swedish() {
        assert_eq!(format_duration_sv(0), "0 min");
        assert_eq!(format_duration_sv(45 * 60_000), "45 min");
        assert_eq!(format_duration_sv(60 * 60_000), "1 tim");
        assert_eq!(format_duration_sv(90 * 60_000), "1 tim 30 min");
    }
}
