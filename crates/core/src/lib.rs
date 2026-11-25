//! Lokal kärna för Tidskvitto. Ingen nätverksåtkomst, inga fönstertitlar
//! lämnar den här koden av sig själv — anroparen bestämmer vad som visas
//! och vad som eventuellt skickas vidare.

mod aggregate;
mod chat;
mod classify;
mod db;
mod insights;
mod settings;
mod timeutil;

pub use aggregate::{
    ai_payload, build_receipt, format_duration_sv, format_percent_sv, top_titles, AiTitles, AppTotal,
    CategoryTotal, DayBucket, Receipt, Suggestion, TitleStat,
};
pub use chat::{plan_question, render_local_answer, Focus, QuestionPlan};
pub use classify::{classify, normalize_title, Category};
pub use db::{Database, Segment};
pub use insights::{build_insights, AgentStart, CategoryTrend, InsightWeek, Insights, TrendCall};
pub use settings::{
    clamp_rate, endpoint_host, resolved_base_url, resolved_model, validate_base_url, Assumptions,
    Settings, ASSUMPTION_CAP, DEFAULT_EMAIL_RATE,
};
pub use timeutil::{monday_of, parse_anchor, span_for, RangeKind, Span};

pub const TAGLINE: &str = "En dag mer frihet. Med AI.";

pub fn estimate_note() -> &'static str {
    "Varje kategori multipliceras med ett antagande du kan ändra. Standard är 40 % av e-post, 20 % av koordinering, 15 % av fokusarbete, 10 % av möten och 0 % av övrigt. Det beskriver hur stor del en AI-agent skulle kunna ta över — inte tid som redan sparats. Taket är 80 % per kategori."
}

pub fn estimate_basis() -> &'static str {
    "Standardvärdet 40 % för e-post ligger i intervallet 30–50 % som 4days.ai brukar nämna för repetitiva uppgifter. Det är ett antagande, inte ett resultat från din logg eller från andras piloter."
}
