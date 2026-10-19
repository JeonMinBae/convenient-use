import { describe, expect, it } from "vitest";
import { acceleratorKeys, keyLabel, toAccelerator } from "./accelerator";

const ev = (code: string, m: Partial<Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>> = {}) =>
  ({ ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, code, ...m });

describe("toAccelerator", () => {
  it("orders modifiers like the Rust normalizer", () => {
    expect(toAccelerator(ev("ArrowRight", { metaKey: true, altKey: true, ctrlKey: true }))).toBe("Ctrl+Alt+Super+ArrowRight");
  });
  it("waits while only modifiers are pressed", () => {
    expect(toAccelerator(ev("ShiftLeft", { shiftKey: true }))).toBeNull();
  });
  it("uses physical code so Korean IME input still yields KeyV", () => {
    expect(toAccelerator(ev("KeyV", { ctrlKey: true, shiftKey: true }))).toBe("Ctrl+Shift+KeyV");
  });
});

describe("keyLabel", () => {
  it("strips Key/Digit prefixes", () => {
    expect(keyLabel("KeyV")).toBe("V");
    expect(keyLabel("Digit1")).toBe("1");
    expect(keyLabel("Numpad5")).toBe("Num 5");
  });
  it("uses arrows and side-specific modifier names", () => {
    expect(keyLabel("ArrowLeft")).toBe("←");
    expect(keyLabel("AltRight")).toBe("Right Alt");
    expect(keyLabel("Super")).toBe("Win");
  });
  it("falls back to the code itself", () => {
    expect(keyLabel("F13")).toBe("F13");
    expect(keyLabel("CapsLock")).toBe("CapsLock");
  });
});

describe("acceleratorKeys", () => {
  it("splits modifiers and key into labels", () => {
    expect(acceleratorKeys("Ctrl+Alt+Super+ArrowRight")).toEqual(["Ctrl", "Alt", "Win", "→"]);
  });
  it("returns empty for an unset hotkey", () => {
    expect(acceleratorKeys("")).toEqual([]);
  });
});
