use chrono::{Datelike, Duration, NaiveDate, TimeZone};

use crate::aggregate::{build_receipt, format_duration_sv, format_percent_sv};
use crate::classify::Category;
use crate::settings::Assumptions;
use crate::timeutil::{monday_of, span_for, RangeKind};
use crate::Segment;

const WEEK_COUNT: usize = 6;
const FLAT_MS: i64 = 5 * 60 * 1000;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightWeek {
    pub label: String,
    pub anchor_date: String,
    pub active_ms: u64,
    pub current: bool,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTrend {
    pub category: Category,
    pub label: String,
    pub weeks_ms: Vec<u64>,
    pub previous_ms: u64,
    pub latest_ms: u64,
    pub delta_ms: i64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendCall {
    pub category: Category,
    pub label: String,
    pub text: String,
    pub delta_ms: i64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStart {
    pub category: Category,
    pub label: String,
    pub text: String,
    pub estimated_savings_ms: u64,
    pub basis: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Insights {
    pub weeks: Vec<InsightWeek>,
    pub trends: Vec<CategoryTrend>,
    pub grown: Option<TrendCall>,
    pub shrunk: Option<TrendCall>,
    pub agent_start: Option<AgentStart>,
    pub comparison_label: String,
    pub needs_more_weeks: bool,
}

pub fn build_insights<Tz>(
    segments: &[Segment],
    today: NaiveDate,
    tz: &Tz,
    assumptions: &Assumptions,
) -> Insights
where
    Tz: TimeZone,
    Tz::Offset: std::fmt::Display,
{
    let assumptions = assumptions.clamped();
    let this_monday = monday_of(today);
    let mut weeks = Vec::with_capacity(WEEK_COUNT);
    let mut by_category: Vec<(Category, Vec<u64>)> = Category::all()
        .into_iter()
        .map(|category| (category, Vec::with_capacity(WEEK_COUNT)))
        .collect();
    let mut savings_by_week: Vec<Vec<(Category, u64, u64)>> = Vec::with_capacity(WEEK_COUNT);

    for back in (0..WEEK_COUNT).rev() {
        let monday = this_monday - Duration::days((back * 7) as i64);
        let span = span_for(RangeKind::Week, monday, tz);
        let receipt = build_receipt(segments, &span, &assumptions);
        let complete = monday + Duration::days(6) < today;
        weeks.push(InsightWeek {
            label: format!("v. {}", monday.iso_week().week()),
            anchor_date: span.anchor_date,
            active_ms: receipt.active_ms,
            current: back == 0,
            complete,
        });
        for (category, series) in &mut by_category {
            let ms = receipt
                .categories
                .iter()
                .find(|item| item.category == *category)
                .map(|item| item.active_ms)
                .unwrap_or(0);
            series.push(ms);
        }
        savings_by_week.push(
            receipt
                .categories
                .iter()
                .map(|item| (item.category, item.active_ms, item.estimated_savings_ms))
                .collect(),
        );
    }

    let complete_indexes: Vec<usize> = weeks
        .iter()
        .enumerate()
        .filter(|(_, week)| week.complete && week.active_ms > 0)
        .map(|(index, _)| index)
        .collect();

    let pair = if complete_indexes.len() >= 2 {
        Some((
            complete_indexes[complete_indexes.len() - 2],
            complete_indexes[complete_indexes.len() - 1],
        ))
    } else {
        None
    };

    let trends = by_category
        .iter()
        .map(|(category, series)| {
            let (previous_ms, latest_ms) = pair
                .map(|(prev, latest)| (series[prev], series[latest]))
                .unwrap_or((0, 0));
            CategoryTrend {
                category: *category,
                label: category.label().to_string(),
                weeks_ms: series.clone(),
                previous_ms,
                latest_ms,
                delta_ms: latest_ms as i64 - previous_ms as i64,
            }
        })
        .collect::<Vec<_>>();

    let comparison_label = pair
        .map(|(prev, latest)| format!("{} mot {}", weeks[prev].label, weeks[latest].label))
        .unwrap_or_default();

    let grown = pair.and_then(|(prev, latest)| {
        trends
            .iter()
            .filter(|trend| trend.delta_ms > FLAT_MS)
            .max_by_key(|trend| trend.delta_ms)
            .map(|trend| TrendCall {
                category: trend.category,
                label: trend.label.clone(),
                delta_ms: trend.delta_ms,
                text: format!(
                    "{} ökade med {} mellan {} och {}.",
                    trend.label,
                    format_duration_sv(trend.delta_ms as u64),
                    weeks[prev].label,
                    weeks[latest].label
                ),
            })
    });
    let shrunk = pair.and_then(|(prev, latest)| {
        trends
            .iter()
            .filter(|trend| trend.delta_ms < -FLAT_MS)
            .min_by_key(|trend| trend.delta_ms)
            .map(|trend| TrendCall {
                category: trend.category,
                label: trend.label.clone(),
                delta_ms: trend.delta_ms,
                text: format!(
                    "{} minskade med {} mellan {} och {}.",
                    trend.label,
                    format_duration_sv((-trend.delta_ms) as u64),
                    weeks[prev].label,
                    weeks[latest].label
                ),
            })
    });

    let agent_index = complete_indexes.last().copied().or_else(|| {
        weeks
            .iter()
            .rposition(|week| week.active_ms > 0)
    });
    let agent_start = agent_index.and_then(|index| {
        let (category, active_ms, savings_ms) = savings_by_week[index]
            .iter()
            .filter(|(_, active, _)| *active > 0)
            .max_by_key(|(_, _, savings)| *savings)?;
        if *savings_ms == 0 && *active_ms == 0 {
            return None;
        }
        let rate = assumptions.for_category(*category);
        let when = if weeks[index].complete {
            weeks[index].label.clone()
        } else {
            format!("{} (pågår)", weeks[index].label)
        };
        Some(AgentStart {
            category: *category,
            label: category.label().to_string(),
            estimated_savings_ms: *savings_ms,
            text: format!(
                "Börja med {}. {} var den {}. Med antagandet {} skulle en agent kunna ta ungefär {} av den tiden. Det är en uppskattning, inte en mätning.",
                category.label().to_lowercase(),
                when,
                format_duration_sv(*active_ms),
                format_percent_sv(rate),
                format_duration_sv(*savings_ms)
            ),
            basis: "Grund: tid i kategorin den senaste veckan med data, gånger antagandet. Ingen mejltext har lästs.".to_string(),
        })
    });

    Insights {
        needs_more_weeks: pair.is_none(),
        weeks,
        trends,
        grown,
        shrunk,
        agent_start,
        comparison_label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Segment;
    use chrono::FixedOffset;

    fn tz() -> FixedOffset {
        FixedOffset::east_opt(3600).unwrap()
    }

    fn seg(start: i64, minutes: i64, category: &str) -> Segment {
        Segment {
            id: start,
            started_at: start,
            ended_at: start + minutes * 60_000,
            app_name: "Test".into(),
            window_title: String::new(),
            category: category.into(),
            idle: false,
        }
    }

    #[test]
    fn trends_compare_the_last_two_complete_weeks_and_agent_follows_savings() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 24).unwrap();
        let older = span_for(
            RangeKind::Week,
            NaiveDate::from_ymd_opt(2026, 11, 9).unwrap(),
            &tz(),
        );
        let newer = span_for(
            RangeKind::Week,
            NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(),
            &tz(),
        );
        let segments = vec![
            seg(older.start_ms + 8 * 3_600_000, 100, "epost"),
            seg(older.start_ms + 12 * 3_600_000, 30, "fokus"),
            seg(newer.start_ms + 8 * 3_600_000, 40, "epost"),
            seg(newer.start_ms + 12 * 3_600_000, 200, "fokus"),
        ];
        let insights = build_insights(&segments, today, &tz(), &Assumptions::default());
        assert!(insights.weeks.iter().any(|week| week.current && !week.complete));
        let focus = insights
            .trends
            .iter()
            .find(|trend| trend.category == Category::Fokus)
            .unwrap();
        assert!(focus.delta_ms > 0);
        assert_eq!(insights.grown.as_ref().unwrap().category, Category::Fokus);
        assert_eq!(insights.shrunk.as_ref().unwrap().category, Category::Epost);
        let start = insights.agent_start.unwrap();
        assert_eq!(start.category, Category::Fokus);
        assert!(start.text.contains("uppskattning"));
        assert!(start.basis.contains("antagandet"));
    }

    #[test]
    fn one_week_is_not_called_a_trend() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 18).unwrap();
        let week = span_for(RangeKind::Week, today, &tz());
        let segments = vec![seg(week.start_ms + 9 * 3_600_000, 50, "epost")];
        let insights = build_insights(&segments, today, &tz(), &Assumptions::default());
        assert!(insights.needs_more_weeks);
        assert!(insights.grown.is_none());
        assert!(insights.shrunk.is_none());
        assert_eq!(
            insights.agent_start.unwrap().category,
            Category::Epost
        );
    }
}
