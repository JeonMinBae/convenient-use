import type { ClipItem } from "../api";

export type ClipKind = "text" | "link" | "image" | "files";

// 공백 없는 http(s) URL 하나로만 된 텍스트. 목록 text 는 300자 미리보기라 긴 URL 은 잘려도 링크로 본다
const URL_ONLY = /^https?:\/\/\S+$/i;

export function clipKind(c: ClipItem): ClipKind {
  if (c.kind !== "text") return c.kind;
  return URL_ONLY.test((c.text ?? "").trim()) ? "link" : "text";
}

const DAY = 86_400_000;

/** 24시간 안은 분·시간, 그 밖에서 전날이면 어제, 올해면 M월 D일, 아니면 YYYY-MM-DD */
export function relativeTime(ms: number, now: number): string {
  const diff = now - ms;
  if (diff < 60_000) return "방금";
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}분 전`;
  if (diff < DAY) return `${Math.floor(diff / 3_600_000)}시간 전`;
  const d = new Date(ms);
  const today = new Date(now);
  const yesterday = new Date(today.getFullYear(), today.getMonth(), today.getDate() - 1);
  if (d.getFullYear() === yesterday.getFullYear() && d.getMonth() === yesterday.getMonth() && d.getDate() === yesterday.getDate())
    return "어제";
  if (d.getFullYear() === today.getFullYear()) return `${d.getMonth() + 1}월 ${d.getDate()}일`;
  const p = (v: number) => String(v).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}
