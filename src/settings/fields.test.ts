import { describe, expect, it } from "vitest";
import { isDirty, pageOfField, pagesWithIssues } from "./fields";

describe("pageOfField", () => {
  it("maps field prefixes to settings pages", () => {
    expect(pageOfField("launchItems.abc.hotkey")).toBe("launch");
    expect(pageOfField("windowHotkeys.leftHalf")).toBe("window");
    expect(pageOfField("clipboard.hotkey")).toBe("clipboard");
    expect(pageOfField("snippets.x.keyword")).toBe("snippet");
    expect(pageOfField("remap.rules.0.trigger")).toBe("remap");
    expect(pageOfField("general.autostart")).toBe("general");
  });
  it("returns null for errors without a field (store errors)", () => {
    expect(pageOfField("")).toBeNull();
    expect(pageOfField("unknown.x")).toBeNull();
  });
});

describe("pagesWithIssues", () => {
  it("unions validation errors and hotkey failures", () => {
    const pages = pagesWithIssues({ errors: { "snippets.a.keyword": "x", "": "store" }, failures: { "clipboard.hotkey": "y" } });
    expect([...pages].sort()).toEqual(["clipboard", "snippet"]);
  });
});

describe("isDirty", () => {
  const s = { general: { autostart: true }, snippets: [{ id: "a", keyword: ";a", text: "" }] } as never;
  it("is false for the same content in a new object", () => {
    expect(isDirty(structuredClone(s), s)).toBe(false);
  });
  it("detects a nested change", () => {
    const d = structuredClone(s) as { snippets: { text: string }[] };
    d.snippets[0].text = "hi";
    expect(isDirty(d as never, s)).toBe(true);
  });
  it("is false before settings are loaded", () => {
    expect(isDirty(null, null)).toBe(false);
  });
});
