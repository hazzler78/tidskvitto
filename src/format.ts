export function formatDuration(ms: number): string {
  const totalMin = Math.round(ms / 60_000);
  if (totalMin < 60) return `${totalMin} min`;
  const hours = Math.floor(totalMin / 60);
  const minutes = totalMin % 60;
  if (minutes === 0) return `${hours} tim`;
  return `${hours} tim ${minutes} min`;
}

export function formatPercent(share: number): string {
  const pct = Number.isFinite(share) ? Math.round(Math.min(1, Math.max(0, share)) * 100) : 0;
  return `${pct}\u00a0%`;
}

export function todayIso(now = new Date()): string {
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

export function parseIso(iso: string): Date {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(y, (m ?? 1) - 1, d ?? 1);
}

export function shiftIso(iso: string, days: number): string {
  const date = parseIso(iso);
  date.setDate(date.getDate() + days);
  return todayIso(date);
}

export function keyStorageLabel(storage: string): string {
  if (storage === "keyring") return "systemets nyckelring";
  if (storage === "database") return "den lokala databasen";
  return "ingen nyckel sparad";
}
