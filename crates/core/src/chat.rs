use chrono::{DateTime, Duration, NaiveDate, TimeZone, Weekday};

use crate::aggregate::{format_duration_sv, format_percent_sv, Receipt};
use crate::timeutil::{most_recent, RangeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Overview,
    Email,
    Meetings,
    Coordination,
    Top,
    Savings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionPlan {
    pub kind: RangeKind,
    pub anchor: NaiveDate,
    pub focus: Focus,
    pub recognized: bool,
}

const HINT: &str = "Utan AI-nyckel svarar jag på frågor om tid per dag eller vecka, e-post, möten och vad som tog mest tid. Prova \"Vad tog mest tid förra veckan?\" eller \"Hur mycket mejlade jag i tisdags?\". En nyckel i Inställningar gör att jag kan resonera friare kring samma siffror — fortfarande bara sammanfattningen.";

pub fn plan_question<Tz>(question: &str, now: DateTime<Tz>) -> QuestionPlan
where
    Tz: TimeZone,
{
    let folded = fold(question);
    let today = now.date_naive();
    let (kind, anchor, period_hit) = resolve_period(&folded, today);
    let (focus, focus_hit) = resolve_focus(&folded);
    QuestionPlan {
        kind,
        anchor,
        focus,
        recognized: period_hit || focus_hit,
    }
}

pub fn render_local_answer(plan: &QuestionPlan, receipt: &Receipt) -> String {
    let body = if receipt.active_ms == 0 && receipt.idle_ms == 0 {
        format!(
            "Jag har ingen uppmätt tid för {}. Antingen var mätningen pausad, eller så var appen inte igång.",
            receipt.period_label
        )
    } else {
        match plan.focus {
            Focus::Email => email_text(receipt),
            Focus::Meetings => category_text(receipt, "Möten", receipt.meeting_ms),
            Focus::Coordination => {
                category_text(receipt, "Koordinering", receipt.coordination_ms)
            }
            Focus::Top => top_text(receipt),
            Focus::Savings => savings_text(receipt),
            Focus::Overview => overview_text(receipt),
        }
    };
    if plan.recognized {
        body
    } else {
        format!("{body}\n\n{HINT}")
    }
}

fn email_text(receipt: &Receipt) -> String {
    format!(
        "Under {} lade du {} på e-post, av {} aktiv tid ({}). Tomgång på {} räknas inte. Uppskattningen för hela perioden, inte bara e-post, är {} och är ett antagande.",
        receipt.period_label,
        format_duration_sv(receipt.email_ms),
        format_duration_sv(receipt.active_ms),
        format_percent_sv(receipt.email_share),
        format_duration_sv(receipt.idle_ms),
        format_duration_sv(receipt.estimated_savings_ms)
    )
}

fn category_text(receipt: &Receipt, label: &str, ms: u64) -> String {
    format!(
        "Under {} är {} {}, av {} aktiv tid ({}).",
        receipt.period_label,
        label.to_lowercase(),
        format_duration_sv(ms),
        format_duration_sv(receipt.active_ms),
        format_percent_sv(if receipt.active_ms == 0 {
            0.0
        } else {
            ms as f64 / receipt.active_ms as f64
        })
    )
}

fn top_text(receipt: &Receipt) -> String {
    let mut cats = receipt.categories.clone();
    cats.sort_by(|a, b| b.active_ms.cmp(&a.active_ms));
    let top_cats = cats
        .iter()
        .filter(|c| c.active_ms > 0)
        .take(3)
        .map(|c| format!("{} {}", c.label.to_lowercase(), format_duration_sv(c.active_ms)))
        .collect::<Vec<_>>();
    let top_apps = receipt
        .apps
        .iter()
        .take(3)
        .map(|a| format!("{} {}", a.app_name, format_duration_sv(a.active_ms)))
        .collect::<Vec<_>>();
    let cats = if top_cats.is_empty() {
        "ingen kategori".to_string()
    } else {
        top_cats.join(", ")
    };
    let apps = if top_apps.is_empty() {
        "inga appar".to_string()
    } else {
        top_apps.join(", ")
    };
    format!(
        "Under {} var den aktiva tiden {}. Mest tid: {cats}. Apparna i topp: {apps}.",
        receipt.period_label,
        format_duration_sv(receipt.active_ms)
    )
}

fn savings_text(receipt: &Receipt) -> String {
    format!(
        "Uppskattningen för {} är {}. Det är inte tid som redan sparats. {}",
        receipt.period_label,
        format_duration_sv(receipt.estimated_savings_ms),
        receipt.estimate_note
    )
}

fn overview_text(receipt: &Receipt) -> String {
    format!(
        "Under {} är den aktiva tiden {}. E-post är {} ({}), koordinering {} och möten {}. Uppskattad tid en agent skulle kunna ta: {} — ett antagande, inte en mätning.",
        receipt.period_label,
        format_duration_sv(receipt.active_ms),
        format_duration_sv(receipt.email_ms),
        format_percent_sv(receipt.email_share),
        format_duration_sv(receipt.coordination_ms),
        format_duration_sv(receipt.meeting_ms),
        format_duration_sv(receipt.estimated_savings_ms)
    )
}

fn resolve_focus(folded: &str) -> (Focus, bool) {
    if contains_any(folded, &["mejl", "e-post", "epost", "mail"]) {
        return (Focus::Email, true);
    }
    if contains_any(folded, &["mote", "moten"]) {
        return (Focus::Meetings, true);
    }
    if folded.contains("koordin") {
        return (Focus::Coordination, true);
    }
    if contains_any(
        folded,
        &["mest tid", "tog mest", "var tiden", "vart tiden", "topp"],
    ) {
        return (Focus::Top, true);
    }
    if contains_any(folded, &["bespar", "uppskatt", "agenten", "agent"]) {
        return (Focus::Savings, true);
    }
    (Focus::Overview, false)
}

fn resolve_period(folded: &str, today: NaiveDate) -> (RangeKind, NaiveDate, bool) {
    if folded.contains("forra veckan") || folded.contains("senaste veckan") {
        let monday = crate::monday_of(today) - Duration::days(7);
        return (RangeKind::Week, monday, true);
    }
    const PAST: &[(&str, Weekday)] = &[
        ("i mandags", Weekday::Mon),
        ("i tisdags", Weekday::Tue),
        ("i onsdags", Weekday::Wed),
        ("i torsdags", Weekday::Thu),
        ("i fredags", Weekday::Fri),
        ("i lordags", Weekday::Sat),
        ("i sondags", Weekday::Sun),
    ];
    for (phrase, weekday) in PAST {
        if folded.contains(phrase) {
            return (RangeKind::Day, most_recent(today, *weekday, false), true);
        }
    }
    const NAMES: &[(&str, Weekday)] = &[
        ("mandag", Weekday::Mon),
        ("tisdag", Weekday::Tue),
        ("onsdag", Weekday::Wed),
        ("torsdag", Weekday::Thu),
        ("fredag", Weekday::Fri),
        ("lordag", Weekday::Sat),
        ("sondag", Weekday::Sun),
    ];
    for (name, weekday) in NAMES {
        if folded.contains(name) {
            let include_today = !folded.contains("forra");
            return (
                RangeKind::Day,
                most_recent(today, *weekday, include_today),
                true,
            );
        }
    }
    if folded.contains("igar") || folded.contains("i gar") {
        return (RangeKind::Day, today - Duration::days(1), true);
    }
    if folded.contains("idag") || folded.contains("i dag") {
        return (RangeKind::Day, today, true);
    }
    if folded.contains("denna vecka")
        || folded.contains("den har veckan")
        || folded.contains("har i veckan")
    {
        return (RangeKind::Week, today, true);
    }
    (RangeKind::Week, today, false)
}

fn contains_any(hay: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| hay.contains(n))
}

fn fold(value: &str) -> String {
    value
        .chars()
        .map(|c| match c.to_lowercase().next().unwrap_or(c) {
            'å' | 'ä' => 'a',
            'ö' => 'o',
            'é' => 'e',
            other => other,
        })
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-')
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate::build_receipt;
    use crate::settings::Assumptions;
    use crate::timeutil::span_for;
    use crate::Segment;
    use chrono::{FixedOffset, TimeZone};

    fn now_wed() -> DateTime<FixedOffset> {
        FixedOffset::east_opt(3600)
            .unwrap()
            .with_ymd_and_hms(2026, 11, 18, 15, 0, 0)
            .unwrap()
    }

    #[test]
    fn tuesday_email_question_points_at_yesterday_when_today_is_wednesday() {
        let plan = plan_question("Hur mycket mejlade jag i tisdags?", now_wed());
        assert!(plan.recognized);
        assert_eq!(plan.focus, Focus::Email);
        assert_eq!(plan.kind, RangeKind::Day);
        assert_eq!(plan.anchor, NaiveDate::from_ymd_opt(2026, 11, 17).unwrap());
    }

    #[test]
    fn last_week_top_question() {
        let plan = plan_question("Vad tog mest tid förra veckan?", now_wed());
        assert_eq!(plan.focus, Focus::Top);
        assert_eq!(plan.kind, RangeKind::Week);
        assert_eq!(plan.anchor, NaiveDate::from_ymd_opt(2026, 11, 9).unwrap());
    }

    #[test]
    fn unknown_question_is_marked_and_still_gets_an_overview() {
        let plan = plan_question("hej hur mår du?", now_wed());
        assert!(!plan.recognized);
        assert_eq!(plan.focus, Focus::Overview);
        let tz = FixedOffset::east_opt(3600).unwrap();
        let span = span_for(plan.kind, plan.anchor, &tz);
        let segment = Segment {
            id: 1,
            started_at: span.start_ms + 8 * 3_600_000,
            ended_at: span.start_ms + 9 * 3_600_000,
            app_name: "Outlook".into(),
            window_title: String::new(),
            category: "epost".into(),
            idle: false,
        };
        let receipt = build_receipt(&[segment], &span, &Assumptions::default());
        let answer = render_local_answer(&plan, &receipt);
        assert!(answer.contains("Utan AI-nyckel"));
        assert!(answer.contains("1 tim"));
    }

    #[test]
    fn tuesday_on_a_tuesday_means_a_week_ago() {
        let now = FixedOffset::east_opt(3600)
            .unwrap()
            .with_ymd_and_hms(2026, 11, 24, 9, 0, 0)
            .unwrap();
        let plan = plan_question("Hur mycket mejlade jag i tisdags?", now);
        assert_eq!(plan.anchor, NaiveDate::from_ymd_opt(2026, 11, 17).unwrap());
    }
}
