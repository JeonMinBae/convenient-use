import type { HotkeyFailure, ValidationError } from "../api";
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
