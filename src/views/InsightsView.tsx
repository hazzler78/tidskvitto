import { useEffect, useState } from "react";
import { getInsights } from "../api";
import { formatDuration } from "../format";
import type { Insights } from "../types";

export function InsightsView({ refreshKey }: { refreshKey: number }) {
  const [insights, setInsights] = useState<Insights | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    let cancelled = false;
    getInsights()
      .then((next) => {
        if (!cancelled) {
          setInsights(next);
          setError("");
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) setError(err instanceof Error ? err.message : "Kunde inte läsa insikterna.");
      });
    return () => {
      cancelled = true;
    };
  }, [refreshKey]);

  return (
    <section>
      <p className="eyebrow">Insikter</p>
      <h1>Hur veckorna rör sig</h1>
      <p className="sub">
        Kvittot är en bild av en period. Här syns vad som växt, vad som minskat, och var en agent skulle kunna börja.
      </p>
      {error ? <p className="error">{error}</p> : null}
      {!insights ? (
        <p>Hämtar insikter…</p>
      ) : (
        <>
          {insights.needsMoreWeeks ? (
            <p className="sub">Det behövs minst två veckor med tid innan en ökning eller minskning syns.</p>
          ) : null}
          <div className="insight-grid">
            <article className="card">
              <h3>Det som växt</h3>
              <p>{insights.grown?.text ?? "Ingen kategori har ökat tydligt mellan de två senaste hela veckorna."}</p>
            </article>
            <article className="card">
              <h3>Det som minskat</h3>
              <p>{insights.shrunk?.text ?? "Ingen kategori har minskat tydligt mellan de två senaste hela veckorna."}</p>
            </article>
            <article className="card suggestion">
              <h3>Där en agent kan börja</h3>
              <p>{insights.agentStart?.text ?? "Det finns ingen tid att utgå från ännu."}</p>
              {insights.agentStart ? <p className="basis">{insights.agentStart.basis}</p> : null}
            </article>
          </div>
          <section className="card" style={{ marginTop: 16 }}>
            <h3>Sex veckor</h3>
            <div className="trend-table">
              <div className="trend-row head">
                <span>Kategori</span>
                {insights.weeks.map((week) => (
                  <span key={week.anchorDate}>{week.current ? `${week.label}*` : week.label}</span>
                ))}
              </div>
              {insights.trends.map((trend) => (
                <div className="trend-row" key={trend.category}>
                  <span>{trend.label}</span>
                  {trend.weeksMs.map((ms, index) => (
                    <b key={`${trend.category}-${insights.weeks[index]?.anchorDate}`}>{formatDuration(ms)}</b>
                  ))}
                </div>
              ))}
            </div>
            <p className="fine">* Veckan pågår. Jämförelsen använder de två senaste hela veckorna{insights.comparisonLabel ? `: ${insights.comparisonLabel}` : ""}.</p>
          </section>
        </>
      )}
    </section>
  );
}