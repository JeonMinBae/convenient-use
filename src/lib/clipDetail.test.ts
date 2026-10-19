import { describe, expect, it } from "vitest";
import type { ClipDetail } from "../api";
import { formatBytes, formatTime, sizeLabel } from "./clipDetail";

const base: ClipDetail = { id: 1, kind: "text", text: null, files: null, image: null,
  imageWidth: null, imageHeight: null, byteSize: null, createdAt: 0, lastUsedAt: 0 };

describe("sizeLabel", () => {
  it("counts emoji as one char and CRLF as one line break", () => {
    expect(sizeLabel({ ...base, text: "a😀\r\nb" })).toBe("5자 · 2줄");
  });
  it("shows image pixels and bytes", () => {
    expect(sizeLabel({ ...base, kind: "image", imageWidth: 30, imageHeight: 20, byteSize: 2048 })).toBe("30×20 · 2.0 KB");
  });
  it("shows file count", () => {
    expect(sizeLabel({ ...base, kind: "files", files: ["a", "b"] })).toBe("파일 2개");
  });
});

describe("formatBytes / formatTime", () => {
  it("switches units", () => {
    expect(formatBytes(1023)).toBe("1023 B");
    expect(formatBytes(10 * 1024 * 1024)).toBe("10.0 MB");
  });
  it("pads local time", () => {
    expect(formatTime(new Date(2026, 0, 2, 3, 4, 5).getTime())).toBe("2026-01-02 03:04:05");
  });
});
