import { useState } from "react";
import { ask } from "../api";
import type { ChatReply } from "../types";

interface Message {
  role: "user" | "bot";
  text: string;
  source?: ChatReply["source"];
  usedTitles?: boolean;
}

const EXAMPLES = [
  "Vad tog mest tid förra veckan?",
  "Hur mycket mejlade jag i tisdags?",
  "Hur stor är uppskattningen den här veckan?",
];

export function ChatView({
  draft,
  hasKey,
  endpointHost,
  shareTitles,
}: {
  draft: string;
  hasKey: boolean;
  endpointHost: string;
  shareTitles: boolean;
}) {
  const [messages, setMessages] = useState<Message[]>([
    {
      role: "bot",
      text: "Fråga om din egen logg. Utan API-nyckel svarar jag direkt från siffrorna. Med nyckel skickas bara en sammanfattning.",
      source: "lokal",
    },
  ]);
  const [text, setText] = useState(draft);
  const [busy, setBusy] = useState(false);

  async function send(question: string) {
    const trimmed = question.trim();
    if (!trimmed || busy) return;
    setMessages((prev) => [...prev, { role: "user", text: trimmed }]);
    setText("");
    setBusy(true);
    try {
      const reply = await ask(trimmed);
      setMessages((prev) => [
        ...prev,
        { role: "bot", text: reply.text, source: reply.source, usedTitles: reply.usedTitles },
      ]);
    } catch (err) {
      setMessages((prev) => [
        ...prev,
        {
          role: "bot",
          text: err instanceof Error ? err.message : "Frågan gick inte att besvara.",
          source: "lokal",
        },
      ]);
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="chat">
      <div>
        <p className="eyebrow">Chatt</p>
        <h1>Fråga loggen</h1>
        <p className="sub">
          {hasKey
            ? `En fråga skickar en sammanfattning till ${endpointHost}. ${
                shareTitles
                  ? "Du har valt att även skicka förkortade fönstertitlar."
                  : "Fönstertitlar ingår inte."
              }`
            : "Ingen nyckel är sparad, så inget lämnar datorn. Svaren räknas lokalt."}
        </p>
        <div className="chips" aria-label="Exempelfrågor">
          {EXAMPLES.map((example) => (
            <button key={example} type="button" onClick={() => send(example)}>
              {example}
            </button>
          ))}
        </div>
        <div className="log" aria-live="polite" style={{ marginTop: 16 }}>
          {messages.map((message, index) => (
            <div key={`${message.role}-${index}`} className={`bubble ${message.role}`}>
              {message.text}
              {message.role === "bot" && message.source ? (
                <span className="meta">
                  {message.source === "ai" ? "Svar från AI-leverantör" : "Lokalt svar"}
                  {message.usedTitles ? " · titlar ingick" : ""}
                </span>
              ) : null}
            </div>
          ))}
        </div>
      </div>
      <form
        className="composer"
        onSubmit={(event) => {
          event.preventDefault();
          void send(text);
        }}
      >
        <label className="sr" htmlFor="question">
          Fråga
        </label>
        <input
          id="question"
          value={text}
          onChange={(event) => setText(event.target.value)}
          placeholder="Till exempel: hur mycket mejlade jag i tisdags?"
          maxLength={2000}
        />
        <button className="primary" type="submit" disabled={busy}>
          {busy ? "Tänker…" : "Skicka"}
        </button>
      </form>
    </section>
  );
}
