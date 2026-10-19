import { useState } from "react";
import { setHotkeysPaused } from "../api";
import { acceleratorKeys, toAccelerator } from "../lib/accelerator";
import { KeyField } from "./Keycap";

interface Props {
  value: string;
  onChange: (v: string) => void;
  error?: string;
  // 전역 단축키 OS 등록 실패 사유 (선점 등)
  failure?: string;
}

export function HotkeyInput({ value, onChange, error, failure }: Props) {
  const [recording, setRecording] = useState(false);
  // 녹화 중에는 전역 단축키를 모두 풀어 Ctrl+Alt+← 같은 조합이 창을 움직이지 않게 한다
  const start = () => {
    setRecording(true);
    setHotkeysPaused(true).catch(() => {});
  };
  const stop = () => {
    setRecording(false);
    setHotkeysPaused(false).catch(() => {});
  };
  return (
    <span className="hotkey-wrap">
      <KeyField
        keys={acceleratorKeys(value)}
        recording={recording}
        placeholder="미지정"
        error={error}
        title={error ?? failure}
        onFocus={start}
        onBlur={stop}
        onKeyDown={(e) => {
          e.preventDefault();
          if (e.key === "Escape") {
            e.currentTarget.blur();
            return;
          }
          // 모디파이어 없는 Backspace/Delete 는 단축키 지우기
          if ((e.key === "Backspace" || e.key === "Delete") && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) {
            onChange("");
            e.currentTarget.blur();
            return;
          }
          const accel = toAccelerator(e.nativeEvent);
          if (accel) {
            onChange(accel);
            e.currentTarget.blur();
          }
        }}
      />
      {failure && (
        <span className="badge" title={failure}>
          등록 실패
        </span>
      )}
      {error && <span className="field-error">{error}</span>}
    </span>
  );
}
