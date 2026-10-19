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

/** KeyboardEvent.code 또는 accelerator 수정자 이름을 키캡에 쓸 짧은 라벨로 바꾼다 */
export function keyLabel(code: string): string {
  if (KEY_LABELS[code]) return KEY_LABELS[code];
  const m = /^(?:Key|Digit)(.)$/.exec(code);
  if (m) return m[1];
  if (code.startsWith("Numpad")) return `Num ${code.slice(6)}`;
  return code;
}

/** "Ctrl+Shift+KeyV" → ["Ctrl", "Shift", "V"]. 빈 문자열은 미지정 */
export const acceleratorKeys = (accel: string): string[] => (accel ? accel.split("+").map(keyLabel) : []);
