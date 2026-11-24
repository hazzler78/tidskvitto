import { useEffect, useState } from "react";
import { getSettings, getStatus, inDesktop, restoreDemo, saveSettings } from "./api";
import { CATEGORY_LABEL, type Category, type HostStatus, type PublicSettings } from "./types";
import { ChatView } from "./views/ChatView";
import { ReceiptView } from "./views/ReceiptView";
import { SettingsView } from "./views/SettingsView";

type View = "kvitto" | "chatt" | "installningar";

const EMPTY_STATUS: HostStatus = {
  os: "browser",
  screenRecording: "not_required",
  idleDetection: "unavailable",
  dataDir: "",
  trackingEnabled: false,
  onboarded: false,
  lastApp: "",
  lastCategory: "",
  lastSampleAt: 0,
  hasApiKey: false,
  endpointHost: "",
  shareTitlesWithAi: false,
};

export function App() {
  const [view, setView] = useState<View>("kvitto");
  const [status, setStatus] = useState<HostStatus>(EMPTY_STATUS);
  const [settings, setSettings] = useState<PublicSettings | null>(null);
  const [draft, setDraft] = useState("");
  const [refreshKey, setRefreshKey] = useState(0);
  const desktop = inDesktop();

  useEffect(() => {
    let cancelled = false;
    getSettings()
      .then((next) => {
        if (!cancelled) setSettings(next);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    const load = () => {
      getStatus()
        .then((next) => {
          if (!cancelled) setStatus(next);
        })
        .catch(() => undefined);
    };
    load();
    const timer = window.setInterval(load, 4000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  useEffect(() => {
    const timer = window.setInterval(() => setRefreshKey((value) => value + 1), 8000);
    return () => window.clearInterval(timer);
  }, []);

  async function setTracking(enabled: boolean) {
    const next = await saveSettings({ trackingEnabled: enabled, onboarded: true });
    setSettings(next);
    setStatus((prev) => ({ ...prev, trackingEnabled: enabled, onboarded: true }));
    setRefreshKey((value) => value + 1);
  }

  const category =
    status.lastCategory in CATEGORY_LABEL
      ? CATEGORY_LABEL[status.lastCategory as Category]
      : "";

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <div className="mark" aria-hidden="true">
            4
          </div>
          <div>
            <strong>Tidskvitto</strong>
            <span>4 Days AI AB</span>
          </div>
        </div>
        <nav className="nav" aria-label="Huvudmeny">
          {(
            [
              ["kvitto", "Kvitto"],
              ["chatt", "Chatt"],
              ["installningar", "Inställningar"],
            ] as const
          ).map(([id, label]) => (
            <button key={id} type="button" aria-current={view === id ? "page" : undefined} onClick={() => setView(id)}>
              {label}
            </button>
          ))}
        </nav>
        <div className="track">
          <span>{status.trackingEnabled ? "Mätning pågår" : "Mätning pausad"}</span>
          <button
            type="button"
            className="switch"
            role="switch"
            aria-checked={status.trackingEnabled}
            aria-label="Mätning"
            onClick={() => void setTracking(!status.trackingEnabled)}
          >
            <i />
          </button>
        </div>
        <p className="tagline">En dag mer frihet. Med AI.</p>
        <div className={status.shareTitlesWithAi ? "privacy-pill warn" : "privacy-pill"}>
          <strong>Allt lagras lokalt</strong>
          {status.hasApiKey
            ? `En fråga kan skicka en sammanfattning till ${status.endpointHost}.`
            : "Inget lämnar datorn förrän du själv kopierar en sammanfattning eller slår på en AI-nyckel."}
          {status.shareTitlesWithAi ? " Fönstertitlar kan följa med i frågor." : ""}
        </div>
      </aside>
      <main className="main">
        {!desktop ? (
          <div className="banner">
            <span>Förhandsvisning med exempeldata. Mätningen och AI-anrop körs i skrivbordsappen, och inget lämnar den här vyn.</span>
            <button
              type="button"
              className="ghost tiny"
              onClick={() => {
                restoreDemo();
                setRefreshKey((value) => value + 1);
              }}
            >
              Återställ exempel
            </button>
          </div>
        ) : null}
        {status.os === "macos" && status.screenRecording === "denied" ? (
          <p className="banner">
            macOS har inte gett skärminspelning. Appnamn mäts ändå. Fönstertitlar, och därmed webbmejl, kräver behörigheten. Se Inställningar.
          </p>
        ) : null}
        {status.trackingEnabled && status.lastApp ? (
          <p className="now">
            Just nu: {status.lastApp}
            {category ? ` · ${category}` : ""}
          </p>
        ) : null}
        {view === "kvitto" ? (
          <ReceiptView
            refreshKey={refreshKey}
            onAsk={(question) => {
              setDraft(question);
              setView("chatt");
            }}
          />
        ) : null}
        {view === "chatt" ? (
          <ChatView
            key={draft}
            draft={draft}
            hasKey={status.hasApiKey}
            endpointHost={status.endpointHost}
            shareTitles={status.shareTitlesWithAi}
          />
        ) : null}
        {view === "installningar" && settings ? (
          <SettingsView
            settings={settings}
            status={status}
            onChange={(next) => {
              setSettings(next);
              setStatus((prev) => ({
                ...prev,
                trackingEnabled: next.trackingEnabled,
                onboarded: next.onboarded,
                hasApiKey: next.hasApiKey,
                endpointHost: next.endpointHost,
                shareTitlesWithAi: next.shareTitlesWithAi,
              }));
              setRefreshKey((value) => value + 1);
            }}
          />
        ) : null}
      </main>
      {settings && !settings.onboarded ? (
        <div className="overlay">
          <div className="dialog" role="dialog" aria-modal="true" aria-labelledby="welcome-title">
            <p className="eyebrow">Innan första provet</p>
            <h2 id="welcome-title">Mätningen stannar här</h2>
            <p>
              Tidskvitto tittar på vilken app och vilket fönster som är aktivt, och hur länge. Det läser inte mejl, filer eller tangenttryckningar.
            </p>
            <p>
              Allt sparas i en databas på den här datorn. Ingenting skickas förrän du själv kopierar en sammanfattning, eller sparar en AI-nyckel och ställer en fråga. Då skickas summerade tider, inte fönstertitlar, om du inte uttryckligen ber om det.
            </p>
            <div className="actions">
              <button type="button" className="primary" onClick={() => void setTracking(true)}>
                Jag förstår, börja mäta
              </button>
              <button type="button" className="ghost" onClick={() => void setTracking(false)}>
                Inte nu
              </button>
            </div>
          </div>
        </div>
      ) : null}
      <style>{`.sr{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0)}`}</style>
    </div>
  );
}
