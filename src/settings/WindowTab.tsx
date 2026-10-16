import type { LayoutCommand, Settings } from "../api";
import { HotkeyInput } from "../components/HotkeyInput";
import type { FieldMaps } from "./fields";

export const LAYOUT_LABELS: Record<LayoutCommand, string> = {
  leftHalf: "왼쪽 절반",
  rightHalf: "오른쪽 절반",
  topHalf: "위쪽 절반",
  bottomHalf: "아래쪽 절반",
  topLeft: "왼쪽 위 1/4",
  topRight: "오른쪽 위 1/4",
  bottomLeft: "왼쪽 아래 1/4",
  bottomRight: "오른쪽 아래 1/4",
  firstThird: "첫 번째 1/3",
  centerThird: "가운데 1/3",
  lastThird: "마지막 1/3",
  firstTwoThirds: "앞쪽 2/3",
  lastTwoThirds: "뒤쪽 2/3",
  maximize: "최대화",
  center: "가운데",
  nextDisplay: "다음 모니터",
};

interface Props {
  draft: Settings;
  onChange: (s: Settings) => void;
  maps: FieldMaps;
}

export function WindowTab({ draft, onChange, maps }: Props) {
  return (
    <section>
      {draft.windowHotkeys.map((w, i) => {
        const field = `windowHotkeys.${w.command}`;
        return (
          <div className="row" key={w.command}>
            <span className="label">{LAYOUT_LABELS[w.command] ?? w.command}</span>
            <HotkeyInput
              value={w.hotkey}
              error={maps.errors[field]}
              failure={maps.failures[field]}
              onChange={(hotkey) =>
                onChange({ ...draft, windowHotkeys: draft.windowHotkeys.map((x, j) => (j === i ? { ...x, hotkey } : x)) })
              }
            />
          </div>
        );
      })}
    </section>
  );
}
