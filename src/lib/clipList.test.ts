import { describe, expect, it } from "vitest";
import type { ClipItem } from "../api";
import { clipKind, relativeTime } from "./clipList";

const base: ClipItem = { id: 1, kind: "text", text: null, files: null, thumbnail: null, pinned: false, lastUsedAt: 0 };

describe("clipKind", () => {
  it("treats a single http(s) URL as a link, ignoring surrounding whitespace", () => {
    expect(clipKind({ ...base, text: "  https://example.com/a?b=1 \n" })).toBe("link");
    expect(clipKind({ ...base, text: "HTTP://EXAMPLE.COM" })).toBe("link");
  });
  it("keeps text with a URL inside or other schemes as text", () => {
    expect(clipKind({ ...base, text: "see https://example.com" })).toBe("text");
    expect(clipKind({ ...base, text: "https://a.com\nhttps://b.com" })).toBe("text");
    expect(clipKind({ ...base, text: "ftp://example.com" })).toBe("text");
    expect(clipKind({ ...base, text: null })).toBe("text");
  });
  it("passes image and files through", () => {
    expect(clipKind({ ...base, kind: "image" })).toBe("image");
    expect(clipKind({ ...base, kind: "files", files: ["C:\\a.txt"] })).toBe("files");
  });
});

describe("relativeTime", () => {
  const now = new Date(2026, 9, 19, 15, 0, 0).getTime();
  const at = (...a: [number, number, number, number, number]) => new Date(...a).getTime();
  it("shows 방금 under a minute and for future timestamps", () => {
    expect(relativeTime(now - 59_000, now)).toBe("방금");
    expect(relativeTime(now + 5_000, now)).toBe("방금");
  });
  it("shows minutes and hours within 24h", () => {
    expect(relativeTime(now - 3 * 60_000, now)).toBe("3분 전");
    expect(relativeTime(now - 2 * 3_600_000, now)).toBe("2시간 전");
    expect(relativeTime(at(2026, 9, 18, 16, 0), now)).toBe("23시간 전");
  });
  it("shows 어제 for the previous calendar day beyond 24h", () => {
    expect(relativeTime(at(2026, 9, 18, 9, 0), now)).toBe("어제");
  });
  it("shows month/day this year and full date otherwise", () => {
    expect(relativeTime(at(2026, 9, 10, 9, 0), now)).toBe("10월 10일");
    expect(relativeTime(at(2025, 11, 31, 9, 0), now)).toBe("2025-12-31");
  });
});
