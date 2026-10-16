import { describe, expect, it } from "vitest";
import { toAccelerator } from "./accelerator";

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
