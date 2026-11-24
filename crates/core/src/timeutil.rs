use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Weekday};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeKind {
    Day,
    Week,
}

impl RangeKind {
    pub fn parse(value: &str) -> Self {
        if value == "week" {
            RangeKind::Week
        } else {
            RangeKind::Day
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RangeKind::Day => "day",
            RangeKind::Week => "week",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayWindow {
    pub date: NaiveDate,
    pub label: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub kind: RangeKind,
    pub anchor: NaiveDate,
    pub start_ms: i64,
    pub end_ms: i64,
    pub period_label: String,
    pub anchor_date: String,
    pub timezone: String,
    pub days: Vec<DayWindow>,
}

pub fn monday_of(date: NaiveDate) -> NaiveDate {
    let back = date.weekday().num_days_from_monday() as i64;
    date - Duration::days(back)
}

pub fn span_for<Tz>(kind: RangeKind, anchor: NaiveDate, tz: &Tz) -> Span
where
    Tz: TimeZone,
    Tz::Offset: std::fmt::Display,
{
    let start_date = match kind {
        RangeKind::Day => anchor,
        RangeKind::Week => monday_of(anchor),
    };
    let end_date = match kind {
        RangeKind::Day => anchor + Duration::days(1),
        RangeKind::Week => start_date + Duration::days(7),
    };
    let start = start_of_day(start_date, tz);
    let end = start_of_day(end_date, tz);
    let mut days = Vec::new();
    let mut cursor = start_date;
    while cursor < end_date {
        let next = cursor + Duration::days(1);
        let day_start = start_of_day(cursor, tz);
        let day_end = start_of_day(next, tz);
        days.push(DayWindow {
            date: cursor,
            label: short_day(cursor),
            start_ms: day_start.timestamp_millis(),
            end_ms: day_end.timestamp_millis(),
        });
        cursor = next;
    }
    Span {
        kind,
        anchor,
        start_ms: start.timestamp_millis(),
        end_ms: end.timestamp_millis(),
        period_label: period_label(kind, start_date, end_date - Duration::days(1)),
        anchor_date: anchor.format("%Y-%m-%d").to_string(),
        timezone: start.format("%Z").to_string(),
        days,
    }
}

fn start_of_day<Tz>(date: NaiveDate, tz: &Tz) -> DateTime<Tz>
where
    Tz: TimeZone,
{
    let naive = date.and_time(NaiveTime::MIN);
    tz.from_local_datetime(&naive)
        .single()
        .or_else(|| tz.from_local_datetime(&naive).earliest())
        .unwrap_or_else(|| tz.from_utc_datetime(&naive))
}

pub fn period_label(kind: RangeKind, start: NaiveDate, end_inclusive: NaiveDate) -> String {
    match kind {
        RangeKind::Day => format!(
            "{} {} {} {}",
            weekday_name(start.weekday()),
            start.day(),
            month_name(start.month()),
            start.year()
        ),
        RangeKind::Week => {
            let week = start.iso_week().week();
            let dates = if start.month() == end_inclusive.month()
                && start.year() == end_inclusive.year()
            {
                format!(
                    "{}–{} {} {}",
                    start.day(),
                    end_inclusive.day(),
                    month_name(start.month()),
                    start.year()
                )
            } else if start.year() == end_inclusive.year() {
                format!(
                    "{} {} – {} {} {}",
                    start.day(),
                    short_month(start.month()),
                    end_inclusive.day(),
                    short_month(end_inclusive.month()),
                    end_inclusive.year()
                )
            } else {
                format!(
                    "{} {} {} – {} {} {}",
                    start.day(),
                    short_month(start.month()),
                    start.year(),
                    end_inclusive.day(),
                    short_month(end_inclusive.month()),
                    end_inclusive.year()
                )
            };
            format!("vecka {week} · {dates}")
        }
    }
}

pub fn short_day(date: NaiveDate) -> String {
    format!("{} {}", weekday_short(date.weekday()), date.day())
}

pub fn weekday_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "måndag",
        Weekday::Tue => "tisdag",
        Weekday::Wed => "onsdag",
        Weekday::Thu => "torsdag",
        Weekday::Fri => "fredag",
        Weekday::Sat => "lördag",
        Weekday::Sun => "söndag",
    }
}

pub fn weekday_short(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "mån",
        Weekday::Tue => "tis",
        Weekday::Wed => "ons",
        Weekday::Thu => "tors",
        Weekday::Fri => "fre",
        Weekday::Sat => "lör",
        Weekday::Sun => "sön",
    }
}

pub fn month_name(month: u32) -> &'static str {
    match month {
        1 => "januari",
        2 => "februari",
        3 => "mars",
        4 => "april",
        5 => "maj",
        6 => "juni",
        7 => "juli",
        8 => "augusti",
        9 => "september",
        10 => "oktober",
        11 => "november",
        _ => "december",
    }
}

pub fn short_month(month: u32) -> &'static str {
    match month {
        1 => "jan",
        2 => "feb",
        3 => "mar",
        4 => "apr",
        5 => "maj",
        6 => "jun",
        7 => "jul",
        8 => "aug",
        9 => "sep",
        10 => "okt",
        11 => "nov",
        _ => "dec",
    }
}

pub fn parse_anchor(value: &str, today: NaiveDate) -> NaiveDate {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").unwrap_or(today)
}

pub fn most_recent(today: NaiveDate, weekday: Weekday, include_today: bool) -> NaiveDate {
    let mut cursor = if include_today {
        today
    } else {
        today - Duration::days(1)
    };
    for _ in 0..8 {
        if cursor.weekday() == weekday {
            return cursor;
        }
        cursor -= Duration::days(1);
    }
    today
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    fn tz() -> FixedOffset {
        FixedOffset::east_opt(3600).unwrap()
    }

    #[test]
    fn week_starts_on_monday_and_covers_seven_midnights() {
        let anchor = NaiveDate::from_ymd_opt(2026, 11, 24).unwrap();
        assert_eq!(anchor.weekday(), Weekday::Tue);
        let span = span_for(RangeKind::Week, anchor, &tz());
        assert_eq!(span.days.len(), 7);
        assert_eq!(span.days[0].date, NaiveDate::from_ymd_opt(2026, 11, 23).unwrap());
        assert_eq!(span.days[6].date, NaiveDate::from_ymd_opt(2026, 11, 29).unwrap());
        assert!(span.period_label.contains("vecka"));
        assert!(span.end_ms > span.start_ms);
    }

    #[test]
    fn in_tisdags_on_a_tuesday_is_seven_days_earlier() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 24).unwrap();
        let found = most_recent(today, Weekday::Tue, false);
        assert_eq!(found, NaiveDate::from_ymd_opt(2026, 11, 17).unwrap());
    }
}
