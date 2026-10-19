import type { ClipDetail } from "../api";

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

/** 텍스트: 글자 수·줄 수 / 이미지: 가로×세로·용량 / 파일: 개수 */
export function sizeLabel(d: ClipDetail): string {
  if (d.kind === "text") {
    const t = d.text ?? "";
    // 서로게이트 쌍(이모지 등)을 한 글자로 센다
    return `${[...t].length.toLocaleString()}자 · ${t.split(/\r\n|\r|\n/).length.toLocaleString()}줄`;
  }
  if (d.kind === "image") {
    const dim = d.imageWidth != null && d.imageHeight != null ? `${d.imageWidth}×${d.imageHeight}` : "크기 알 수 없음";
    return d.byteSize != null ? `${dim} · ${formatBytes(d.byteSize)}` : dim;
  }
  return `파일 ${(d.files ?? []).length}개`;
}

export function formatTime(ms: number): string {
  const d = new Date(ms);
  const p = (v: number) => String(v).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}
