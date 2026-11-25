import { useState, type FormEvent } from "react";
import { deleteEverything, deleteMeasurements, requestScreenPermission, saveSettings } from "../api";
import { keyStorageLabel } from "../format";
import type { Assumptions, HostStatus, PublicSettings } from "../types";

export function SettingsView({
  settings,
  status,
  onChange,
}: {
  settings: PublicSettings;
  status: HostStatus;
  onChange: (next: PublicSettings) => void;
}) {
  const [draft, setDraft] = useState(settings);
  const [apiKey, setApiKey] = useState("");
  const [message, setMessage] = useState("");
  const [confirm, setConfirm] = useState<"data" | "all" | null>(null);

  function assumption(key: keyof Assumptions, percent: number) {
    setDraft({ ...draft, assumptions: { ...draft.assumptions, [key]: percent / 100 } });
  }

  async function save(event: FormEvent) {
    event.preventDefault();
    setMessage("");
    try {
      const next = await saveSettings({
        idleThresholdSecs: draft.idleThresholdSecs,
        storeWindowTitles: draft.storeWindowTitles,
        shareTitlesWithAi: draft.shareTitlesWithAi,
        aiProvider: draft.aiProvider,
        aiBaseUrl: draft.aiBaseUrl,
        aiModel: draft.aiModel,
        autostart: draft.autostart,
        assumptions: draft.assumptions,
        apiKey: apiKey.length > 0 ? apiKey : null,
      });
      setDraft(next);
      setApiKey("");
      onChange(next);
      setMessage("Sparat på den här datorn.");
    } catch (err) {
      setMessage(err instanceof Error ? err.message : "Kunde inte spara.");
    }
  }

  return (
    <section>
      <p className="eyebrow">Inställningar</p>
      <h1>Integritet och antaganden</h1>
      <p className="sub">All mätdata ligger i {status.dataDir}.</p>
      {message ? <p className="banner">{message}</p> : null}
      <form className="settings" onSubmit={save}>
        <section className="card">
          <h3>Mätning</h3>
          <label className="field">
            <span>Tomgång efter minuter utan inmatning</span>
            <input
              type="number"
              min={1}
              max={30}
              value={Math.round(draft.idleThresholdSecs / 60)}
              onChange={(event) =>
                setDraft({ ...draft, idleThresholdSecs: Number(event.target.value) * 60 })
              }
            />
          </label>
          <label className="field">
            <span>
              <input
                type="checkbox"
                checked={draft.autostart}
                onChange={(event) => setDraft({ ...draft, autostart: event.target.checked })}
              />{" "}
              Starta Tidskvitto när jag loggar in
            </span>
          </label>
          <p className="fine">
            Att stänga fönstret pausar inte mätningen. Appen ligger kvar i menyraden tills du pausar eller väljer Avsluta.
          </p>
        </section>

        <section className="card">
          <h3>Vad som sparas</h3>
          <label className="field">
            <span>
              <input
                type="checkbox"
                checked={draft.storeWindowTitles}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    storeWindowTitles: event.target.checked,
                    shareTitlesWithAi: event.target.checked ? draft.shareTitlesWithAi : false,
                  })
                }
              />{" "}
              Spara fönstertitlar lokalt
            </span>
          </label>
          <p className="fine">
            Titeln kan innehålla ett mejlämne. Den används i minnet för att känna igen webbmejl även om du stänger av lagring, och slängs då direkt. Utan sparade titlar kan inte förslaget om återkommande fönster räknas.
          </p>
          <label className="field">
            <span>
              <input
                type="checkbox"
                checked={draft.shareTitlesWithAi}
                disabled={!draft.storeWindowTitles}
                onChange={(event) => setDraft({ ...draft, shareTitlesWithAi: event.target.checked })}
              />{" "}
              Skicka förkortade fönstertitlar när jag frågar AI:n
            </span>
          </label>
          <p className="fine">Av som standard. Mejladresser i titeln byts mot [mejl] innan något skickas.</p>
        </section>

        <section className="card">
          <h3>Behörighet</h3>
          <div className="help">
            {status.os === "macos" ? (
              <MacHelp denied={status.screenRecording !== "granted"} />
            ) : (
              <>
                <p>
                  På Windows behövs ingen extra behörighet för aktivt fönster. På macOS kallar systemet det skärminspelning, även när appen bara läser fönstrets titel. Tidskvitto tar inga skärmbilder.
                </p>
                <details open>
                  <summary>Så gör du på macOS</summary>
                  <MacSteps />
                </details>
              </>
            )}
            {status.idleDetection === "unavailable" ? (
              <p>Tomgång kunde inte läsas på den här datorn, så all tid framför ett fönster räknas som aktiv.</p>
            ) : null}
          </div>
        </section>

        <section className="card">
          <h3>AI-leverantör</h3>
          <p className="fine">
            Kvittot fungerar utan nyckel. Nyckeln sparas i {keyStorageLabel(draft.keyStorage)} och skickas bara till värden du anger här.
          </p>
          <div className="grid-2">
            <label className="field">
              <span>Leverantör</span>
              <select
                value={draft.aiProvider}
                onChange={(event) => setDraft({ ...draft, aiProvider: event.target.value })}
              >
                <option value="grok">Grok</option>
                <option value="openai">OpenAI</option>
                <option value="anthropic">Anthropic</option>
                <option value="compatible">OpenAI-kompatibel</option>
              </select>
            </label>
            <label className="field">
              <span>Modell, tomt ger {draft.resolvedModel}</span>
              <input
                value={draft.aiModel}
                onChange={(event) => setDraft({ ...draft, aiModel: event.target.value })}
                placeholder={draft.resolvedModel}
              />
            </label>
          </div>
          <label className="field">
            <span>Basadress, tomt ger leverantörens standard. Måste vara https.</span>
            <input
              value={draft.aiBaseUrl}
              onChange={(event) => setDraft({ ...draft, aiBaseUrl: event.target.value })}
              placeholder={
                draft.aiProvider === "anthropic"
                  ? "https://api.anthropic.com/v1"
                  : draft.aiProvider === "grok"
                    ? "https://api.x.ai/v1"
                    : "https://api.openai.com/v1"
              }
            />
          </label>
          <label className="field">
            <span>API-nyckel {draft.hasApiKey ? "(sparad, lämna tom för att behålla)" : ""}</span>
            <input
              type="password"
              autoComplete="off"
              value={apiKey}
              onChange={(event) => setApiKey(event.target.value)}
              placeholder={draft.hasApiKey ? "••••••••" : "klistras in lokalt"}
            />
          </label>
          {draft.hasApiKey ? (
            <button
              type="button"
              className="ghost tiny"
              onClick={() => {
                setApiKey("");
                void saveSettings({ apiKey: "" }).then((next) => {
                  setDraft(next);
                  onChange(next);
                  setMessage("Nyckeln är raderad.");
                });
              }}
            >
              Ta bort nyckel
            </button>
          ) : null}
        </section>

        <section className="card">
          <h3>Antaganden för uppskattningen</h3>
          <p className="fine">Andel av tiden i kategorin som en agent antas kunna ta. Taket är 80 %.</p>
          {(
            [
              ["epost", "E-post"],
              ["koordinering", "Koordinering"],
              ["fokus", "Fokusarbete"],
              ["moten", "Möten"],
              ["ovrigt", "Övrigt"],
            ] as const
          ).map(([key, label]) => (
            <label className="slider" key={key}>
              <span>{label}</span>
              <input
                type="range"
                min={0}
                max={80}
                step={5}
                value={Math.round(draft.assumptions[key] * 100)}
                onChange={(event) => assumption(key, Number(event.target.value))}
              />
              <b>{Math.round(draft.assumptions[key] * 100)} %</b>
            </label>
          ))}
        </section>

        <div className="actions">
          <button className="primary" type="submit">
            Spara inställningar
          </button>
        </div>
      </form>

      <section className="card" style={{ marginTop: 16 }}>
        <h3>Radera</h3>
        <p className="fine">Mätdata är segmenten. Inställningar och nyckel ligger kvar om du bara raderar mätningen.</p>
        <div className="actions">
          {confirm === "data" ? (
            <>
              <button
                type="button"
                className="danger"
                onClick={() => {
                  void deleteMeasurements().then(() => {
                    setConfirm(null);
                    setMessage("All mätdata är raderad.");
                  });
                }}
              >
                Ja, radera mätdata
              </button>
              <button type="button" className="ghost" onClick={() => setConfirm(null)}>
                Avbryt
              </button>
            </>
          ) : (
            <button type="button" className="danger" onClick={() => setConfirm("data")}>
              Radera all mätdata
            </button>
          )}
          {confirm === "all" ? (
            <>
              <button
                type="button"
                className="danger"
                onClick={() => {
                  void deleteEverything().then((next) => {
                    setDraft(next);
                    onChange(next);
                    setConfirm(null);
                    setMessage("Allt är raderat, inklusive nyckel och inställningar.");
                  });
                }}
              >
                Ja, radera allt
              </button>
              <button type="button" className="ghost" onClick={() => setConfirm(null)}>
                Avbryt
              </button>
            </>
          ) : (
            <button type="button" className="danger" onClick={() => setConfirm("all")}>
              Radera allt, inklusive nyckel
            </button>
          )}
        </div>
      </section>
    </section>
  );
}

function MacHelp({ denied }: { denied: boolean }) {
  return (
    <>
      <p>
        {denied
          ? "Skärminspelning är inte aktiverad. Appnamn kan ändå mätas. Fönstertitel, som behövs för Gmail och annan webbmejl, kräver behörigheten."
          : "Skärminspelning är aktiverad. Tidskvitto använder den bara för att läsa fönstrets titel, inte för att spara bilder."}
      </p>
      <MacSteps />
      {denied ? (
        <button
          type="button"
          className="primary tiny"
          onClick={() => {
            void requestScreenPermission();
          }}
        >
          Be macOS om behörighet
        </button>
      ) : null}
    </>
  );
}

function MacSteps() {
  return (
    <ol>
      <li>Öppna Systeminställningar.</li>
      <li>Gå till Integritet och säkerhet, sedan Skärm- och systemljudsinspelning.</li>
      <li>Aktivera Tidskvitto. Om appen saknas, lägg till den från Programmappen.</li>
      <li>Starta om Tidskvitto efter att du ändrat behörigheten.</li>
    </ol>
  );
}
