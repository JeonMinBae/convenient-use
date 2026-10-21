import { isMac } from "./platform";

const MODIFIER_CODES = new Set([
  "ControlLeft", "ControlRight", "AltLeft", "AltRight",
  "ShiftLeft", "ShiftRight", "MetaLeft", "MetaRight",
]);

/** 모디파이어만 눌린 상태면 null (녹화 계속). Rust accelerator::normalize 와 같은 순서를 만든다. */
export function toAccelerator(e: Pick<KeyboardEvent, "ctrlKey" | "altKey" | "shiftKey" | "metaKey" | "code">): string | null {
  if (MODIFIER_CODES.has(e.code)) return null;
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Super");
  parts.push(e.code);
  return parts.join("+");
}

const KEY_LABELS: Record<string, string> = {
  ArrowLeft: "←", ArrowRight: "→", ArrowUp: "↑", ArrowDown: "↓",
  Super: "Win", MetaLeft: "Win", MetaRight: "Right Win",
  ControlLeft: "Ctrl", ControlRight: "Right Ctrl", AltLeft: "Alt", AltRight: "Right Alt",
  ShiftLeft: "Shift", ShiftRight: "Right Shift",
  Backquote: "`", Minus: "-", Equal: "=", BracketLeft: "[", BracketRight: "]",
  Backslash: "\\", Semicolon: ";", Quote: "'", Comma: ",", Period: ".", Slash: "/",
};

// macOS 는 Super 가 Command, Alt 가 Option 이다. 메뉴 막대와 같은 기호로 보여준다
const MAC_LABELS: Record<string, string> = {
  Super: "⌘", MetaLeft: "⌘", MetaRight: "Right ⌘",
  Ctrl: "⌃", ControlLeft: "⌃", ControlRight: "Right ⌃",
  Alt: "⌥", AltLeft: "⌥", AltRight: "Right ⌥",
  Shift: "⇧", ShiftLeft: "⇧", ShiftRight: "Right ⇧",
};

/** KeyboardEvent.code 또는 accelerator 수정자 이름을 키캡에 쓸 짧은 라벨로 바꾼다 */
export function keyLabel(code: string, mac = isMac): string {
  if (mac && MAC_LABELS[code]) return MAC_LABELS[code];
  if (KEY_LABELS[code]) return KEY_LABELS[code];
  const m = /^(?:Key|Digit)(.)$/.exec(code);
  if (m) return m[1];
  if (code.startsWith("Numpad")) return `Num ${code.slice(6)}`;
  return code;
}

/** "Ctrl+Shift+KeyV" → ["Ctrl", "Shift", "V"]. 빈 문자열은 미지정 */
export const acceleratorKeys = (accel: string): string[] => (accel ? accel.split("+").map((k) => keyLabel(k)) : []);
