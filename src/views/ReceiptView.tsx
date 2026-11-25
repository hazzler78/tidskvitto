import { useEffect, useState } from "react";
import { getReceipt } from "../api";
import { formatDuration, formatPercent, shiftIso, todayIso } from "../format";
import type { RangeKind, Receipt } from "../types";

export function ReceiptView({
  refreshKey,
  onAsk,
}: {
  refreshKey: number;
  onAsk: (question: string) => void;
}) {
  const [range, setRange] = useState<RangeKind>("day");
  const [anchor, setAnchor] = useState(todayIso());
  const [receipt, setReceipt] = useState<Receipt | null>(null);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getReceipt(range, anchor)
      .then((next) => {
        if (!cancelled) {
          setReceipt(next);
          setError("");
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) setError(err instanceof Error ? err.message : "Kunde inte läsa kvittot.");
      });
    return () => {
      cancelled = true;
    };
  }, [range, anchor, refreshKey]);

  const step = range === "week" ? 7 : 1;
  const nextDisabled = shiftIso(anchor, step) > todayIso();
  const maxDay = Math.max(1, ...(receipt?.days.map((d) => d.activeMs) ?? [1]));

  async function copy() {
    if (!receipt) return;
    try {
      await navigator.clipboard.writeText(receipt.plaintext);
    } catch {
      const area = document.createElement("textarea");
      area.value = receipt.plaintext;
      document.body.appendChild(area);
      area.select();
      document.execCommand("copy");
      area.remove();
    }
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1600);
  }

  return (
    <section>
      <div className="top">
        <div>
          <p className="eyebrow">4days.ai</p>
          <h1>Tidskvitto</h1>
          <p className="sub">Appen uppskattar din tid baserat på vad du gör, inte exakt mätning.</p>
        </div>
        <div className="segmented" role="group" aria-label="Period">
          <button type="button" aria-pressed={range === "day"} onClick={() => setRange("day")}>
            Dag
          </button>
          <button type="button" aria-pressed={range === "week"} onClick={() => setRange("week")}>
            Vecka
          </button>
        </div>
      </div>
      <div className="pager" style={{ marginBottom: 16 }}>
        <button type="button" onClick={() => setAnchor(shiftIso(anchor, -step))} aria-label="Föregående period">
          Föregående
        </button>
        <button
          type="button"
          onClick={() => setAnchor(shiftIso(anchor, step))}
          disabled={nextDisabled}
          aria-label="Nästa period"
        >
          Nästa
        </button>
        <button type="button" className="ghost" onClick={copy} disabled={!receipt}>
          {copied ? "Kopierat" : "Kopiera sammanfattning"}
        </button>
      </div>
      {error ? <p className="error">{error}</p> : null}
      {receipt ? (
        <div className="layout">
          <article className="receipt" aria-label="Kvitto">
            <div className="receipt-head">
              <div>
                <p className="kicker">Tidskvitto</p>
                <h2>KVITTO</h2>
              </div>
              <span className="stamp">LOKALT</span>
            </div>
            <p className="period">{receipt.periodLabel}</p>
            {receipt.activeMs === 0 ? (
              <p className="empty">
                Ingen uppmätt tid den här perioden. Låt appen vara igång medan du arbetar, eller slå på mätningen om den är pausad.
              </p>
            ) : (
              <>
                <hr className="rule" />
                <div className="total">
                  <span>Aktiv tid</span>
                  <b>{formatDuration(receipt.activeMs)}</b>
                </div>
                <div className="row">
                  <span>Tomgång, räknas inte</span>
                  <b>{formatDuration(receipt.idleMs)}</b>
                </div>
                <hr className="rule" />
                {receipt.categories.map((category) => (
                  <div className="row" key={category.category}>
                    <span>
                      <i className={`dot ${category.category}`} />
                      {category.label}
                    </span>
                    <b>{formatDuration(category.activeMs)}</b>
                  </div>
                ))}
                <hr className="rule" />
                <div className="row">
                  <span>E-postandel</span>
                  <b>{formatPercent(receipt.emailShare)}</b>
                </div>
                <div className="row">
                  <span>Koordinering</span>
                  <b>{formatPercent(receipt.coordinationShare)}</b>
                </div>
              </>
            )}
            <hr className="rule" />
            <div className="row">
              <span>En agent skulle kunna ta</span>
              <b>{formatDuration(receipt.estimatedSavingsMs)}</b>
            </div>
            <p className="tag">En dag mer frihet. Med AI.</p>
          </article>
          <div className="panel">
            <section className="card">
              <h3>{range === "week" ? "Dagarna" : "Dagen"}</h3>
              <div className="bars" aria-hidden="true">
                {receipt.days.map((day) => (
                  <div className="bar" key={day.date}>
                    <i
                      style={{
                        height: day.activeMs === 0 ? "3px" : `${Math.max(10, (day.activeMs / maxDay) * 110)}px`,
                        background: day.activeMs === 0 ? "#e2e8f0" : undefined,
                      }}
                    />
                    <span>{day.label}</span>
                  </div>
                ))}
              </div>
            </section>
            <section className="card">
              <h3>Appar</h3>
              {receipt.apps.length === 0 ? (
                <p className="fine">Inga appar i perioden.</p>
              ) : (
                <ul className="apps">
                  {receipt.apps.map((app) => (
                    <li key={`${app.appName}-${app.category}`}>
                      <span>{app.appName}</span>
                      <b>{formatDuration(app.activeMs)}</b>
                    </li>
                  ))}
                </ul>
              )}
            </section>
            <section className="card">
              <h3>Så räknas uppskattningen</h3>
              <ul className="formula">
                {receipt.categories.map((category) => (
                  <li key={category.category}>
                    <span>
                      {category.label} · {formatPercent(category.assumption)}
                    </span>
                    <b>{formatDuration(category.estimatedSavingsMs)}</b>
                  </li>
                ))}
              </ul>
            </section>
            {receipt.suggestions.map((suggestion) => (
              <article className="card suggestion" key={suggestion.id}>
                <h3>{suggestion.title}</h3>
                <p>{suggestion.body}</p>
                <p className="basis">{suggestion.basis}</p>
                <button
                  type="button"
                  className="ghost tiny"
                  onClick={() =>
                    onAsk(
                      "Hur mycket tid i den här perioden ser ut att kunna lämnas till en agent, och vad bygger uppskattningen på?",
                    )
                  }
                >
                  Fråga chatten om det här
                </button>
              </article>
            ))}
          </div>
        </div>
      ) : (
        <p>Hämtar kvitto…</p>
      )}
    </section>
  );
}
