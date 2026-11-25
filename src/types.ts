export type Category = "fokus" | "epost" | "koordinering" | "mote" | "ovrigt";
export type RangeKind = "day" | "week";

export interface Assumptions {
  epost: number;
  koordinering: number;
  moten: number;
  fokus: number;
  ovrigt: number;
}

export interface CategoryTotal {
  category: Category;
  label: string;
  activeMs: number;
  share: number;
  assumption: number;
  estimatedSavingsMs: number;
}

export interface AppTotal {
  appName: string;
  category: Category;
  activeMs: number;
}

export interface DayBucket {
  date: string;
  label: string;
  activeMs: number;
  emailMs: number;
  estimatedSavingsMs: number;
}

export interface Suggestion {
  id: string;
  title: string;
  body: string;
  basis: string;
}

export interface Receipt {
  range: RangeKind;
  anchorDate: string;
  periodLabel: string;
  timezone: string;
  activeMs: number;
  idleMs: number;
  emailMs: number;
  emailShare: number;
  coordinationMs: number;
  coordinationShare: number;
  meetingMs: number;
  categories: CategoryTotal[];
  apps: AppTotal[];
  days: DayBucket[];
  estimatedSavingsMs: number;
  assumptions: Assumptions;
  estimateNote: string;
  estimateBasis: string;
  suggestions: Suggestion[];
  plaintext: string;
}

export interface PublicSettings {
  trackingEnabled: boolean;
  onboarded: boolean;
  idleThresholdSecs: number;
  storeWindowTitles: boolean;
  shareTitlesWithAi: boolean;
  aiProvider: string;
  aiBaseUrl: string;
  aiModel: string;
  autostart: boolean;
  assumptions: Assumptions;
  hasApiKey: boolean;
  keyStorage: string;
  endpointHost: string;
  resolvedModel: string;
}

export interface HostStatus {
  os: string;
  screenRecording: string;
  idleDetection: string;
  dataDir: string;
  trackingEnabled: boolean;
  onboarded: boolean;
  lastApp: string;
  lastCategory: string;
  lastSampleAt: number;
  hasApiKey: boolean;
  endpointHost: string;
  shareTitlesWithAi: boolean;
}

export interface SettingsPatch {
  trackingEnabled?: boolean;
  onboarded?: boolean;
  idleThresholdSecs?: number;
  storeWindowTitles?: boolean;
  shareTitlesWithAi?: boolean;
  aiProvider?: string;
  aiBaseUrl?: string;
  aiModel?: string;
  autostart?: boolean;
  assumptions?: Assumptions;
  apiKey?: string | null;
}

export interface InsightWeek {
  label: string;
  anchorDate: string;
  activeMs: number;
  current: boolean;
  complete: boolean;
}

export interface CategoryTrend {
  category: Category;
  label: string;
  weeksMs: number[];
  previousMs: number;
  latestMs: number;
  deltaMs: number;
}

export interface TrendCall {
  category: Category;
  label: string;
  text: string;
  deltaMs: number;
}

export interface AgentStart {
  category: Category;
  label: string;
  text: string;
  estimatedSavingsMs: number;
  basis: string;
}

export interface Insights {
  weeks: InsightWeek[];
  trends: CategoryTrend[];
  grown: TrendCall | null;
  shrunk: TrendCall | null;
  agentStart: AgentStart | null;
  comparisonLabel: string;
  needsMoreWeeks: boolean;
}

export interface ChatReply {
  text: string;
  source: "lokal" | "ai";
  usedTitles: boolean;
  periodLabel: string;
}

export const ESTIMATE_NOTE =
  "Varje kategori multipliceras med ett antagande du kan ändra. Standard är 40 % av e-post, 20 % av koordinering, 15 % av fokusarbete, 10 % av möten och 0 % av övrigt. Det beskriver hur stor del en AI-agent skulle kunna ta över — inte tid som redan sparats. Taket är 80 % per kategori.";

export const ESTIMATE_BASIS =
  "Standardvärdet 40 % för e-post ligger i intervallet 30–50 % som 4days.ai brukar nämna för repetitiva uppgifter. Det är ett antagande, inte ett resultat från din logg eller från andras piloter.";

export const CATEGORY_ORDER: Category[] = [
  "fokus",
  "epost",
  "koordinering",
  "mote",
  "ovrigt",
];

export const CATEGORY_LABEL: Record<Category, string> = {
  fokus: "Fokusarbete",
  epost: "E-post",
  koordinering: "Koordinering",
  mote: "Möten",
  ovrigt: "Övrigt",
};
