import type { HotkeyFailure, Settings, ValidationError } from "../api";
import { sourceField } from "../api";

export interface FieldMaps {
  errors: Record<string, string>;
  failures: Record<string, string>;
}

export function buildFieldMaps(errors: ValidationError[], failures: HotkeyFailure[]): FieldMaps {
  const e: Record<string, string> = {};
  for (const x of errors) e[x.field] = x.message;
  const f: Record<string, string> = {};
  for (const x of failures) f[sourceField(x.source)] = `${x.hotkey}: ${x.reason}`;
  return { errors: e, failures: f };
}

export const isWindows = navigator.userAgent.includes("Windows");
export const osKey: "windows" | "macos" = isWindows ? "windows" : "macos";

export const clamp = (v: number, min: number, max: number) =>
  Number.isFinite(v) ? Math.min(max, Math.max(min, Math.round(v))) : min;

export type SettingsPage = "general" | "launch" | "window" | "clipboard" | "snippet" | "remap";

// 필드 경로의 첫 세그먼트로 페이지를 정한다. 필드가 없거나 모르는 경로(저장소 오류 등)는 null
const PAGE_BY_PREFIX: Record<string, SettingsPage> = {
  general: "general",
  launchItems: "launch",
  windowHotkeys: "window",
  clipboard: "clipboard",
  snippets: "snippet",
  remap: "remap",
};

export function pageOfField(field: string): SettingsPage | null {
  return PAGE_BY_PREFIX[field.split(".")[0]] ?? null;
}

export function pagesWithIssues(maps: FieldMaps): Set<SettingsPage> {
  const pages = new Set<SettingsPage>();
  for (const f of [...Object.keys(maps.errors), ...Object.keys(maps.failures)]) {
    const p = pageOfField(f);
    if (p) pages.add(p);
  }
  return pages;
}

// 같은 경로로 만든 객체끼리의 비교라 키 순서가 같다고 보고 직렬화로 비교한다
export function isDirty(draft: Settings | null, baseline: Settings | null): boolean {
  if (!draft || !baseline) return false;
  return JSON.stringify(draft) !== JSON.stringify(baseline);
}
