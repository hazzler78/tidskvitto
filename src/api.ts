import { invoke } from "@tauri-apps/api/core";
import {
  demoAsk,
  demoDeleteMeasurements,
  demoReceipt,
  demoReset,
  demoRestore,
  demoSave,
  demoSettings,
  demoStatus,
} from "./demo";
import type {
  ChatReply,
  HostStatus,
  PublicSettings,
  RangeKind,
  Receipt,
  SettingsPatch,
} from "./types";

export function inDesktop(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function getStatus(): Promise<HostStatus> {
  if (!inDesktop()) return demoStatus();
  return invoke<HostStatus>("status");
}

export async function getSettings(): Promise<PublicSettings> {
  if (!inDesktop()) return demoSettings();
  return invoke<PublicSettings>("get_settings");
}

export async function saveSettings(patch: SettingsPatch): Promise<PublicSettings> {
  if (!inDesktop()) return demoSave(patch);
  return invoke<PublicSettings>("save_settings", { patch });
}

export async function getReceipt(range: RangeKind, anchor: string): Promise<Receipt> {
  if (!inDesktop()) return demoReceipt(range, anchor);
  return invoke<Receipt>("receipt", { range, anchor });
}

export async function ask(question: string): Promise<ChatReply> {
  if (!inDesktop()) return demoAsk(question);
  return invoke<ChatReply>("ask", { question });
}

export async function deleteMeasurements(): Promise<void> {
  if (!inDesktop()) {
    demoDeleteMeasurements();
    return;
  }
  await invoke("delete_measurements");
}

export async function deleteEverything(): Promise<PublicSettings> {
  if (!inDesktop()) return demoReset();
  return invoke<PublicSettings>("delete_everything");
}

export async function requestScreenPermission(): Promise<string> {
  if (!inDesktop()) return "not_required";
  return invoke<string>("request_screen_permission");
}

export function restoreDemo() {
  if (!inDesktop()) demoRestore();
}
