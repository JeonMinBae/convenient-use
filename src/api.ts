import { invoke } from "@tauri-apps/api/core";

export interface ClipItem {
  id: number;
  kind: "text" | "image" | "files";
  text: string | null;
  files: string[] | null;
  thumbnail: string | null;
  pinned: boolean;
  lastUsedAt: number;
}

/** 한 페이지는 100개. 목록의 text 는 300자까지만 오는 미리보기다 */
export const listClips = (query: string, offset = 0) => invoke<ClipItem[]>("list_clips", { query, offset });
export const CLIP_PAGE_SIZE = 100;
export interface ClipDetail {
  id: number;
  kind: "text" | "image" | "files";
  text: string | null;
  files: string[] | null;
  image: string | null;
  imageWidth: number | null;
  imageHeight: number | null;
  byteSize: number | null;
  createdAt: number;
  lastUsedAt: number;
}
export const getClipDetail = (id: number) => invoke<ClipDetail | null>("get_clip_detail", { id });
export const setPinned = (id: number, pinned: boolean) => invoke<void>("set_pinned", { id, pinned });
export const deleteClip = (id: number) => invoke<void>("delete_clip", { id });
export const clearClips = () => invoke<void>("clear_clips");
export const pasteClip = (id: number) => invoke<void>("paste_clip", { id });
export const hideClipboardPopup = () => invoke<void>("hide_clipboard_popup");

// ---- 설정 (Rust settings::model 과 같은 모양, serde camelCase) ----

export type LayoutCommand =
  | "leftHalf" | "rightHalf" | "topHalf" | "bottomHalf"
  | "topLeft" | "topRight" | "bottomLeft" | "bottomRight"
  | "firstThird" | "centerThird" | "lastThird" | "firstTwoThirds" | "lastTwoThirds"
  | "maximize" | "center" | "nextDisplay";

export interface OsPath {
  windows?: string | null;
  macos?: string | null;
}

export type LaunchTarget =
  | { type: "app"; path: OsPath }
  | { type: "url"; url: string; browser: OsPath };

export interface LaunchItem {
  id: string;
  name: string;
  hotkey: string;
  target: LaunchTarget;
}

export interface FeatureToggles {
  launch: boolean;
  window: boolean;
  clipboard: boolean;
  snippet: boolean;
  remap: boolean;
}

export interface KeyCombo {
  mods: string[];
  key: string | null;
}

export type RemapRule =
  | { type: "simple"; from: string; to: KeyCombo }
  | { type: "tapHold"; key: string; tap: KeyCombo; hold: KeyCombo }
  | { type: "layer"; trigger: string; tap: KeyCombo | null; mappings: { from: string; to: KeyCombo }[] }
  | { type: "macro"; trigger: string; steps: KeyCombo[] };

export interface Snippet {
  id: string;
  keyword: string;
  text: string;
}

export interface Settings {
  general: { autostart: boolean; features: FeatureToggles };
  launchItems: LaunchItem[];
  windowHotkeys: { command: LayoutCommand; hotkey: string }[];
  clipboard: { hotkey: string; maxItems: number; maxImageMb: number };
  snippets: Snippet[];
  remap: { rules: RemapRule[]; excludedApps: string[] };
}

export interface ValidationError {
  field: string;
  message: string;
}

/** Rust BindingSource (tag "kind", content "id") */
export type BindingSource =
  | { kind: "launch"; id: string }
  | { kind: "window"; id: LayoutCommand }
  | { kind: "clipboard" }
  | { kind: "macro"; id: number };

export interface HotkeyFailure {
  source: BindingSource;
  hotkey: string;
  reason: string;
}

export interface SaveResult {
  hotkeyFailures: HotkeyFailure[];
}

export interface InstalledApp {
  name: string;
  path: string;
}

/** Rust BindingSource::field() 와 같은 규칙. 검증 오류·등록 실패가 한 조회로 합쳐진다. */
export function sourceField(s: BindingSource): string {
  switch (s.kind) {
    case "launch": return `launchItems.${s.id}.hotkey`;
    case "window": return `windowHotkeys.${s.id}`;
    case "clipboard": return "clipboard.hotkey";
    case "macro": return `remap.rules.${s.id}.trigger`;
  }
}

export const getSettings = () => invoke<Settings>("get_settings");
/** 실패 시 ValidationError[] 로 reject 된다 */
export const saveSettings = (settings: Settings) => invoke<SaveResult>("save_settings", { settings });
export const exportSettings = (path: string) => invoke<void>("export_settings", { path });
export const importSettings = (path: string) => invoke<Settings>("import_settings", { path });
export const listInstalledApps = () => invoke<InstalledApp[]>("list_installed_apps");
export const setHotkeysPaused = (paused: boolean) => invoke<void>("set_hotkeys_paused", { paused });
export const getHotkeyFailures = () => invoke<HotkeyFailure[]>("get_hotkey_failures");
