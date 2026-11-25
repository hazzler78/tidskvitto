import { formatDuration, formatPercent, parseIso, todayIso } from "./format";
import {
  CATEGORY_LABEL,
  CATEGORY_ORDER,
  ESTIMATE_BASIS,
  ESTIMATE_NOTE,
  type Assumptions,
  type Category,
  type ChatReply,
  type DayBucket,
  type HostStatus,
  type InsightWeek,
  type Insights,
  type PublicSettings,
  type RangeKind,
  type Receipt,
  type SettingsPatch,
  type Suggestion,
} from "./types";

interface DemoSeg {
  startedAt: number;
  endedAt: number;
  appName: string;
  windowTitle: string;
  category: Category;
  idle: boolean;
}

const defaultAssumptions = (): Assumptions => ({
  epost: 0.4,
  koordinering: 0.2,
  moten: 0.1,
  fokus: 0.15,
  ovrigt: 0,
});

function defaultSettings(): PublicSettings {
  return {
    trackingEnabled: false,
    onboarded: false,
    idleThresholdSecs: 180,
    storeWindowTitles: true,
    shareTitlesWithAi: false,
    aiProvider: "grok",
    aiBaseUrl: "",
    aiModel: "",
    autostart: false,
    assumptions: defaultAssumptions(),
    hasApiKey: false,
    keyStorage: "none",
    endpointHost: "api.x.ai",
    resolvedModel: "grok-4.7",
  };
}

let settings = defaultSettings();
let memoryKey = "";
let segments: DemoSeg[] = seed(new Date());

function seed(now: Date): DemoSeg[] {
  const out: DemoSeg[] = [];
  for (let back = 11; back >= 0; back -= 1) {
    const day = new Date(now);
    day.setDate(now.getDate() - back);
    day.setHours(8, 15, 0, 0);
    const dow = day.getDay();
    if (dow === 0 || dow === 6) continue;
    const cursor = day.getTime();
    const push = (
      minutes: number,
      appName: string,
      category: Category,
      windowTitle = "",
      idle = false,
    ) => {
      const same = out.filter((s) => sameDay(s.startedAt, day.getTime()));
      const start = same.length ? same[same.length - 1]!.endedAt : cursor;
      out.push({
        startedAt: start,
        endedAt: start + minutes * 60_000,
        appName,
        windowTitle,
        category,
        idle,
      });
    };
    const wobble = (back % 3) * 5;
    push(150 + wobble, "Cursor", "fokus");
    push(35, "Google Chrome", "fokus", "Specifikation - Google Docs");
    push(20, "Microsoft Outlook", "epost", "Re: Offert Q4 - kund@bolag.se - Outlook");
    push(15, "Slack", "koordinering", "marknad");
    if (dow === 2 || dow === 4) push(45, "Microsoft Teams", "mote", "Veckomöte");
    push(25, "Microsoft Outlook", "epost", "Inkorg - Outlook");
    push(20, "Slack", "koordinering", "allmänt");
    push(12, "Spotify", "ovrigt");
    push(18, "Microsoft Outlook", "epost", "", true);
  }
  return out;
}

function sameDay(a: number, b: number) {
  const da = new Date(a);
  const db = new Date(b);
  return da.getFullYear() === db.getFullYear() && da.getMonth() === db.getMonth() && da.getDate() === db.getDate();
}

function hostFor(provider: string, base: string): string {
  if (base) {
    try {
      return new URL(base).host;
    } catch {
      return base;
    }
  }
  if (provider === "anthropic") return "api.anthropic.com";
  if (provider === "grok") return "api.x.ai";
  return "api.openai.com";
}

function modelFor(provider: string, model: string): string {
  if (model.trim()) return model.trim();
  if (provider === "anthropic") return "claude-sonnet-4-5";
  if (provider === "grok") return "grok-4.7";
  return "gpt-4o-mini";
}

export function demoStatus(): HostStatus {
  return {
    os: "browser",
    screenRecording: "not_required",
    idleDetection: "unavailable",
    dataDir: "Förhandsvisning — ingen databas",
    trackingEnabled: settings.trackingEnabled,
    onboarded: settings.onboarded,
    lastApp: settings.trackingEnabled ? "Microsoft Outlook" : "",
    lastCategory: settings.trackingEnabled ? "epost" : "",
    lastSampleAt: settings.trackingEnabled ? Date.now() : 0,
    hasApiKey: settings.hasApiKey,
    endpointHost: settings.endpointHost,
    shareTitlesWithAi: settings.shareTitlesWithAi,
  };
}

export function demoSettings(): PublicSettings {
  return structuredClone(settings);
}

export function demoSave(patch: SettingsPatch): PublicSettings {
  settings = {
    ...settings,
    ...pick(patch),
    assumptions: clampAssumptions(patch.assumptions ?? settings.assumptions),
  };
  if (!settings.storeWindowTitles) settings.shareTitlesWithAi = false;
  settings.endpointHost = hostFor(settings.aiProvider, settings.aiBaseUrl);
  settings.resolvedModel = modelFor(settings.aiProvider, settings.aiModel);
  if (patch.apiKey !== undefined && patch.apiKey !== null) {
    memoryKey = patch.apiKey.trim();
    settings.hasApiKey = memoryKey.length > 0;
    settings.keyStorage = settings.hasApiKey ? "database" : "none";
  }
  return demoSettings();
}

function pick(patch: SettingsPatch): Partial<PublicSettings> {
  return {
    trackingEnabled: patch.trackingEnabled ?? settings.trackingEnabled,
    onboarded: patch.onboarded ?? settings.onboarded,
    idleThresholdSecs: Math.min(1800, Math.max(30, patch.idleThresholdSecs ?? settings.idleThresholdSecs)),
    storeWindowTitles: patch.storeWindowTitles ?? settings.storeWindowTitles,
    shareTitlesWithAi: patch.shareTitlesWithAi ?? settings.shareTitlesWithAi,
    aiProvider: patch.aiProvider ?? settings.aiProvider,
    aiBaseUrl: (patch.aiBaseUrl ?? settings.aiBaseUrl).replace(/\/$/, ""),
    aiModel: patch.aiModel ?? settings.aiModel,
    autostart: patch.autostart ?? settings.autostart,
  };
}

function clampAssumptions(value: Assumptions): Assumptions {
  const clamp = (n: number) => (Number.isFinite(n) ? Math.min(0.8, Math.max(0, n)) : 0);
  return {
    epost: clamp(value.epost),
    koordinering: clamp(value.koordinering),
    moten: clamp(value.moten),
    fokus: clamp(value.fokus),
    ovrigt: clamp(value.ovrigt),
  };
}

export function demoDeleteMeasurements() {
  segments = [];
}

export function demoReset(): PublicSettings {
  memoryKey = "";
  settings = defaultSettings();
  segments = [];
  return demoSettings();
}

export function demoRestore() {
  segments = seed(new Date());
}

export function demoReceipt(range: RangeKind, anchorIso: string): Receipt {
  const anchor = parseIso(anchorIso || todayIso());
  const days = range === "week" ? weekDays(anchor) : [startOfDay(anchor)];
  const spanStart = days[0]!.getTime();
  const spanEnd = addDays(days[days.length - 1]!, 1).getTime();
  return build(range, anchor, days, spanStart, spanEnd);
}

export function demoAsk(question: string): ChatReply {
  const planned = planQuestion(question, new Date());
  const receipt = demoReceipt(planned.kind, todayIso(planned.anchor));
  const local = render(planned, receipt);
  if (!settings.hasApiKey) {
    return { text: local, source: "lokal", usedTitles: false, periodLabel: receipt.periodLabel };
  }
  return {
    text: `Förhandsvisningen anropar inte ${settings.endpointHost}, även om en nyckel är ifylld. Svaret nedan är lokalt.\n\n${local}`,
    source: "lokal",
    usedTitles: false,
    periodLabel: receipt.periodLabel,
  };
}

function build(range: RangeKind, anchor: Date, days: Date[], spanStart: number, spanEnd: number): Receipt {
  const assumptions = settings.assumptions;
  const totals = Object.fromEntries(CATEGORY_ORDER.map((c) => [c, 0])) as Record<Category, number>;
  const apps = new Map<string, { category: Category; ms: number }>();
  let idleMs = 0;
  const titles = new Map<string, { display: string; visits: number; ms: number }>();
  for (const seg of segments) {
    const ms = overlap(seg.startedAt, seg.endedAt, spanStart, spanEnd);
    if (ms <= 0) continue;
    if (seg.idle) {
      idleMs += ms;
      continue;
    }
    totals[seg.category] += ms;
    const key = `${seg.appName}|${seg.category}`;
    const prev = apps.get(key);
    apps.set(key, { category: seg.category, ms: (prev?.ms ?? 0) + ms });
    if (seg.category === "epost") addTitle(titles, seg.windowTitle, ms);
  }
  const activeMs = CATEGORY_ORDER.reduce((sum, c) => sum + totals[c], 0);
  const categories = CATEGORY_ORDER.map((category) => {
    const active = totals[category];
    const assumption = assumptions[category === "mote" ? "moten" : category];
    return {
      category,
      label: CATEGORY_LABEL[category],
      activeMs: active,
      share: activeMs ? active / activeMs : 0,
      assumption,
      estimatedSavingsMs: Math.round(active * assumption),
    };
  });
  const estimatedSavingsMs = categories.reduce((s, c) => s + c.estimatedSavingsMs, 0);
  const dayBuckets: DayBucket[] = days.map((day) => {
    const start = day.getTime();
    const end = addDays(day, 1).getTime();
    let active = 0;
    let email = 0;
    let saved = 0;
    const per = Object.fromEntries(CATEGORY_ORDER.map((c) => [c, 0])) as Record<Category, number>;
    for (const seg of segments) {
      const ms = overlap(seg.startedAt, seg.endedAt, start, end);
      if (ms <= 0 || seg.idle) continue;
      active += ms;
      per[seg.category] += ms;
      if (seg.category === "epost") email += ms;
    }
    for (const category of CATEGORY_ORDER) {
      const assumption = assumptions[category === "mote" ? "moten" : category];
      saved += Math.round(per[category] * assumption);
    }
    return {
      date: todayIso(day),
      label: shortDay(day),
      activeMs: active,
      emailMs: email,
      estimatedSavingsMs: saved,
    };
  });
  const appList = [...apps.entries()]
    .map(([key, value]) => ({
      appName: key.split("|")[0]!,
      category: value.category,
      activeMs: value.ms,
    }))
    .sort((a, b) => b.activeMs - a.activeMs)
    .slice(0, 8);
  const emailMs = totals.epost;
  const coordinationMs = totals.koordinering;
  const suggestions = suggest(emailMs, coordinationMs, activeMs, estimatedSavingsMs, titles);
  const receipt: Receipt = {
    range,
    anchorDate: todayIso(anchor),
    periodLabel: periodLabel(range, days),
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone || "lokal tid",
    activeMs,
    idleMs,
    emailMs,
    emailShare: activeMs ? emailMs / activeMs : 0,
    coordinationMs,
    coordinationShare: activeMs ? coordinationMs / activeMs : 0,
    meetingMs: totals.mote,
    categories,
    apps: appList,
    days: dayBuckets,
    estimatedSavingsMs,
    assumptions,
    estimateNote: ESTIMATE_NOTE,
    estimateBasis: ESTIMATE_BASIS,
    suggestions,
    plaintext: "",
  };
  receipt.plaintext = plaintext(receipt);
  const sum = receipt.categories.reduce((s, c) => s + c.activeMs, 0);
  if (sum !== receipt.activeMs) throw new Error("Exempelkvittot summerar inte.");
  return receipt;
}

function suggest(
  emailMs: number,
  coordinationMs: number,
  activeMs: number,
  estimatedMs: number,
  titles: Map<string, { display: string; visits: number; ms: number }>,
): Suggestion[] {
  const out: Suggestion[] = [];
  const repeats = [...titles.values()]
    .filter((t) => t.visits >= 3 && t.ms >= 8 * 60_000)
    .sort((a, b) => b.ms - a.ms);
  for (const [index, title] of repeats.slice(0, 2).entries()) {
    out.push({
      id: `repeat-${index}`,
      title: "Återkommande e-postfönster",
      body: `Fönstret "${title.display}" har varit aktivt ${title.visits} gånger (${formatDuration(title.ms)}). Det kan vara ett återkommande ärende en agent kan förbereda svar till.`,
      basis: `Grund: ${title.visits} besök i e-post med liknande fönstertitel, sammanlagt ${formatDuration(title.ms)}. Tidskvitto har inte läst mejlet och skickar inget.`,
    });
  }
  if (emailMs >= 45 * 60_000 && activeMs && emailMs / activeMs >= 0.2) {
    out.push({
      id: "email-share",
      title: "E-post tar en stor del av tiden",
      body: `E-post är ${formatPercent(emailMs / activeMs)} av den aktiva tiden (${formatDuration(emailMs)}). Med standardantagandet 40 % skulle en agent kunna ta ungefär ${formatDuration(Math.round(emailMs * 0.4))} av den tiden — sortering och utkast, inte beslut.`,
      basis: "Grund: uppmätt tid i mejlprogram och webbmejl, gånger antagandet 40 %. Ingen mejltext har lästs.",
    });
  }
  if (coordinationMs >= 40 * 60_000) {
    out.push({
      id: "coordination",
      title: "Koordinering går att sammanfatta",
      body: `Koordinering (chatt och kalender) är ${formatDuration(coordinationMs)}. En agent kan sammanfatta trådar och föreslå svar, ungefär ${formatDuration(Math.round(coordinationMs * 0.2))} med standardantagandet 20 %.`,
      basis: "Grund: tid i chatt- och kalenderappar, gånger antagandet 20 %.",
    });
  }
  if (estimatedMs >= 60 * 60_000) {
    out.push({
      id: "savings",
      title: "Uppskattningen är värd att titta på",
      body: `Om antagandena stämmer skulle en agent kunna ta ungefär ${formatDuration(estimatedMs)} av den här perioden. Det är inte tid du redan sparat.`,
      basis: "Grund: summan av kategori gånger antagande. Se formeln på kvittot.",
    });
  }
  return out.slice(0, 3);
}

function addTitle(map: Map<string, { display: string; visits: number; ms: number }>, raw: string, ms: number) {
  const normalized = normalize(raw);
  if (normalized.length < 4 || ["inkorg", "gmail", "outlook", "inbox", "mail"].includes(normalized)) return;
  const display = raw.replace(/\S+@\S+\.\S+/g, "[mejl]").slice(0, 80);
  const prev = map.get(normalized) ?? { display, visits: 0, ms: 0 };
  prev.visits += 1;
  prev.ms += ms;
  map.set(normalized, prev);
}

function normalize(title: string): string {
  let s = title.toLowerCase();
  for (const suffix of [" - outlook", " - gmail", " - google chrome"]) {
    if (s.endsWith(suffix)) s = s.slice(0, -suffix.length);
  }
  s = s.replace(/^(re:|sv:|fwd:|fw:)\s*/g, "").trim();
  return s.replace(/\s+/g, " ");
}

function plaintext(receipt: Receipt): string {
  const lines = [
    "TIDSKVITTO",
    "4days.ai",
    receipt.periodLabel,
    "",
    `Aktiv tid: ${formatDuration(receipt.activeMs)}`,
    `Tomgång (räknas inte): ${formatDuration(receipt.idleMs)}`,
    "",
    ...receipt.categories.map((c) => `${c.label}: ${formatDuration(c.activeMs)}`),
    "",
    `E-postandel: ${formatPercent(receipt.emailShare)}`,
    `Koordinering: ${formatPercent(receipt.coordinationShare)}`,
    "",
    `Uppskattad tid en agent skulle kunna ta: ${formatDuration(receipt.estimatedSavingsMs)}`,
    "Det här är en uppskattning, inte en mätning.",
    receipt.estimateNote,
    receipt.estimateBasis,
    "",
    "En dag mer frihet. Med AI.",
  ];
  return lines.join("\n");
}

function overlap(start: number, end: number, winStart: number, winEnd: number) {
  return Math.max(0, Math.min(end, winEnd) - Math.max(start, winStart));
}

function startOfDay(date: Date) {
  const d = new Date(date);
  d.setHours(0, 0, 0, 0);
  return d;
}

function addDays(date: Date, n: number) {
  const d = new Date(date);
  d.setDate(d.getDate() + n);
  return d;
}

function weekDays(anchor: Date) {
  const monday = startOfDay(anchor);
  const day = monday.getDay();
  const diff = day === 0 ? 6 : day - 1;
  monday.setDate(monday.getDate() - diff);
  return Array.from({ length: 7 }, (_, i) => addDays(monday, i));
}

const WEEKDAYS = ["söndag", "måndag", "tisdag", "onsdag", "torsdag", "fredag", "lördag"];
const WEEKDAYS_SHORT = ["sön", "mån", "tis", "ons", "tors", "fre", "lör"];
const MONTHS = [
  "januari",
  "februari",
  "mars",
  "april",
  "maj",
  "juni",
  "juli",
  "augusti",
  "september",
  "oktober",
  "november",
  "december",
];
const MONTHS_SHORT = ["jan", "feb", "mar", "apr", "maj", "jun", "jul", "aug", "sep", "okt", "nov", "dec"];

function shortDay(date: Date) {
  return `${WEEKDAYS_SHORT[date.getDay()]} ${date.getDate()}`;
}

function isoWeek(date: Date) {
  const d = new Date(Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()));
  const day = d.getUTCDay() || 7;
  d.setUTCDate(d.getUTCDate() + 4 - day);
  const yearStart = new Date(Date.UTC(d.getUTCFullYear(), 0, 1));
  return Math.ceil(((d.getTime() - yearStart.getTime()) / 86_400_000 + 1) / 7);
}

function periodLabel(range: RangeKind, days: Date[]) {
  const start = days[0]!;
  const end = days[days.length - 1]!;
  if (range === "day") {
    return `${WEEKDAYS[start.getDay()]} ${start.getDate()} ${MONTHS[start.getMonth()]} ${start.getFullYear()}`;
  }
  const dates =
    start.getMonth() === end.getMonth()
      ? `${start.getDate()}–${end.getDate()} ${MONTHS[start.getMonth()]} ${start.getFullYear()}`
      : `${start.getDate()} ${MONTHS_SHORT[start.getMonth()]} – ${end.getDate()} ${MONTHS_SHORT[end.getMonth()]} ${end.getFullYear()}`;
  return `vecka ${isoWeek(start)} · ${dates}`;
}

interface Plan {
  kind: RangeKind;
  anchor: Date;
  focus: "overview" | "email" | "top" | "savings" | "meetings" | "coordination";
  recognized: boolean;
}

function planQuestion(question: string, now: Date): Plan {
  const folded = question
    .toLowerCase()
    .replaceAll("å", "a")
    .replaceAll("ä", "a")
    .replaceAll("ö", "o");
  let kind: RangeKind = "week";
  let anchor = startOfDay(now);
  let period = false;
  if (folded.includes("forra veckan") || folded.includes("senaste veckan")) {
    kind = "week";
    anchor = addDays(weekDays(now)[0]!, -7);
    period = true;
  } else if (folded.includes("i tisdags")) {
    kind = "day";
    anchor = mostRecent(now, 2, false);
    period = true;
  } else if (folded.includes("igar")) {
    kind = "day";
    anchor = addDays(startOfDay(now), -1);
    period = true;
  } else if (folded.includes("idag") || folded.includes("i dag")) {
    kind = "day";
    period = true;
  }
  let focus: Plan["focus"] = "overview";
  let focusHit = false;
  if (/mejl|e-post|epost|mail/.test(folded)) {
    focus = "email";
    focusHit = true;
  } else if (folded.includes("mest tid") || folded.includes("tog mest")) {
    focus = "top";
    focusHit = true;
  } else if (folded.includes("bespar") || folded.includes("uppskatt") || folded.includes("agent")) {
    focus = "savings";
    focusHit = true;
  } else if (folded.includes("mote")) {
    focus = "meetings";
    focusHit = true;
  } else if (folded.includes("koordin")) {
    focus = "coordination";
    focusHit = true;
  }
  return { kind, anchor, focus, recognized: period || focusHit };
}

function mostRecent(now: Date, weekday: number, includeToday: boolean) {
  const cursor = startOfDay(now);
  if (!includeToday) cursor.setDate(cursor.getDate() - 1);
  for (let i = 0; i < 8; i += 1) {
    if (cursor.getDay() === weekday) return new Date(cursor);
    cursor.setDate(cursor.getDate() - 1);
  }
  return startOfDay(now);
}

function render(plan: Plan, receipt: Receipt): string {
  const body =
    receipt.activeMs === 0
      ? `Jag har ingen uppmätt tid för ${receipt.periodLabel}.`
      : plan.focus === "email"
        ? `Under ${receipt.periodLabel} lade du ${formatDuration(receipt.emailMs)} på e-post, av ${formatDuration(receipt.activeMs)} aktiv tid (${formatPercent(receipt.emailShare)}).`
        : plan.focus === "top"
          ? topText(receipt)
          : plan.focus === "savings"
            ? `Uppskattningen för ${receipt.periodLabel} är ${formatDuration(receipt.estimatedSavingsMs)}. Det är inte tid som redan sparats. ${ESTIMATE_NOTE}`
            : `Under ${receipt.periodLabel} är den aktiva tiden ${formatDuration(receipt.activeMs)}. E-post är ${formatDuration(receipt.emailMs)} (${formatPercent(receipt.emailShare)}). Uppskattad tid en agent skulle kunna ta: ${formatDuration(receipt.estimatedSavingsMs)} — ett antagande, inte en mätning.`;
  if (plan.recognized) return body;
  return `${body}\n\nUtan AI-nyckel svarar jag på frågor om tid per dag eller vecka, e-post, möten och vad som tog mest tid. Prova "Vad tog mest tid förra veckan?" eller "Hur mycket mejlade jag i tisdags?".`;
}

function topText(receipt: Receipt) {
  const cats = [...receipt.categories]
    .filter((c) => c.activeMs > 0)
    .sort((a, b) => b.activeMs - a.activeMs)
    .slice(0, 3)
    .map((c) => `${c.label.toLowerCase()} ${formatDuration(c.activeMs)}`)
    .join(", ");
  const apps = receipt.apps
    .slice(0, 3)
    .map((a) => `${a.appName} ${formatDuration(a.activeMs)}`)
    .join(", ");
  return `Under ${receipt.periodLabel} var den aktiva tiden ${formatDuration(receipt.activeMs)}. Mest tid: ${cats}. Apparna i topp: ${apps}.`;
}

const FLAT_MS = 5 * 60_000;

export function demoInsights(): Insights {
  const today = startOfDay(new Date());
  const thisMonday = weekDays(today)[0]!;
  const weeks: InsightWeek[] = [];
  const series = Object.fromEntries(CATEGORY_ORDER.map((category) => [category, [] as number[]])) as Record<
    Category,
    number[]
  >;
  const savings: { category: Category; activeMs: number; savingsMs: number }[][] = [];
  for (let back = 5; back >= 0; back -= 1) {
    const monday = addDays(thisMonday, -7 * back);
    const receipt = demoReceipt("week", todayIso(monday));
    const sunday = addDays(monday, 6);
    weeks.push({
      label: `v. ${isoWeek(monday)}`,
      anchorDate: todayIso(monday),
      activeMs: receipt.activeMs,
      current: back === 0,
      complete: sunday < today,
    });
    for (const category of CATEGORY_ORDER) {
      series[category].push(receipt.categories.find((item) => item.category === category)?.activeMs ?? 0);
    }
    savings.push(
      receipt.categories.map((item) => ({
        category: item.category,
        activeMs: item.activeMs,
        savingsMs: item.estimatedSavingsMs,
      })),
    );
  }
  const complete = weeks.flatMap((week, index) => (week.complete && week.activeMs > 0 ? [index] : []));
  const pair = complete.length >= 2 ? [complete[complete.length - 2]!, complete[complete.length - 1]!] : null;
  const trends = CATEGORY_ORDER.map((category) => {
    const previousMs = pair ? series[category][pair[0]!]! : 0;
    const latestMs = pair ? series[category][pair[1]!]! : 0;
    return {
      category,
      label: CATEGORY_LABEL[category],
      weeksMs: series[category],
      previousMs,
      latestMs,
      deltaMs: latestMs - previousMs,
    };
  });
  const grownTrend = pair ? trends.filter((trend) => trend.deltaMs > FLAT_MS).sort((a, b) => b.deltaMs - a.deltaMs)[0] : undefined;
  const shrunkTrend = pair ? trends.filter((trend) => trend.deltaMs < -FLAT_MS).sort((a, b) => a.deltaMs - b.deltaMs)[0] : undefined;
  const call = (trend: (typeof trends)[number] | undefined, verb: "ökade" | "minskade") =>
    trend && pair
      ? {
          category: trend.category,
          label: trend.label,
          deltaMs: trend.deltaMs,
          text: `${trend.label} ${verb} med ${formatDuration(Math.abs(trend.deltaMs))} mellan ${weeks[pair[0]!]!.label} och ${weeks[pair[1]!]!.label}.`,
        }
      : null;
  let agentIndex = complete.at(-1) ?? -1;
  if (agentIndex < 0) {
    for (let index = weeks.length - 1; index >= 0; index -= 1) {
      if (weeks[index]!.activeMs > 0) {
        agentIndex = index;
        break;
      }
    }
  }
  const agent = agentIndex >= 0
    ? savings[agentIndex]!.filter((item) => item.activeMs > 0).sort((a, b) => b.savingsMs - a.savingsMs)[0]
    : undefined;
  const agentWeek = agentIndex >= 0 ? weeks[agentIndex] : undefined;
  return {
    weeks,
    trends,
    grown: call(grownTrend, "ökade"),
    shrunk: call(shrunkTrend, "minskade"),
    comparisonLabel: pair ? `${weeks[pair[0]!]!.label} mot ${weeks[pair[1]!]!.label}` : "",
    needsMoreWeeks: pair === null,
    agentStart:
      agent && agentWeek
        ? {
            category: agent.category,
            label: CATEGORY_LABEL[agent.category],
            estimatedSavingsMs: agent.savingsMs,
            text: `Börja med ${CATEGORY_LABEL[agent.category].toLowerCase()}. ${agentWeek.complete ? agentWeek.label : `${agentWeek.label} (pågår)`} var den ${formatDuration(agent.activeMs)}. Med antagandet ${formatPercent(settings.assumptions[agent.category === "mote" ? "moten" : agent.category])} skulle en agent kunna ta ungefär ${formatDuration(agent.savingsMs)} av den tiden. Det är en uppskattning, inte en mätning.`,
            basis: "Grund: tid i kategorin den senaste veckan med data, gånger antagandet. Ingen mejltext har lästs.",
          }
        : null,
  };
}
