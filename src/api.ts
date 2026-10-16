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

export const listClips = (query: string) => invoke<ClipItem[]>("list_clips", { query });
export const setPinned = (id: number, pinned: boolean) => invoke<void>("set_pinned", { id, pinned });
export const deleteClip = (id: number) => invoke<void>("delete_clip", { id });
export const clearClips = () => invoke<void>("clear_clips");
export const pasteClip = (id: number) => invoke<void>("paste_clip", { id });
export const hideClipboardPopup = () => invoke<void>("hide_clipboard_popup");
