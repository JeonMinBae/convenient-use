import { useState } from "react";
import { type Settings, clearClips } from "../api";
import { HotkeyInput } from "../components/HotkeyInput";
import type { Notice } from "../components/InfoBar";
import { SettingsCard } from "../components/SettingsCard";
import { type FieldMaps, clamp } from "./fields";

interface Props {
  draft: Settings;
  onChange: (s: Settings) => void;
  maps: FieldMaps;
  onNotice: (n: Notice) => void;
}

export function ClipboardTab({ draft, onChange, maps, onNotice }: Props) {
  const c = draft.clipboard;
  const set = (patch: Partial<typeof c>) => onChange({ ...draft, clipboard: { ...c, ...patch } });
  // 입력 중 빈 값을 허용하려고 문자열로 들고 있다가 blur 때 범위로 보정한다
  const [maxItems, setMaxItems] = useState(String(c.maxItems));
  const [maxImageMb, setMaxImageMb] = useState(String(c.maxImageMb));
  // 가져오기 등으로 draft 가 바뀌면 표시값을 맞춘다
  const [prev, setPrev] = useState(c);
  if (prev.maxItems !== c.maxItems || prev.maxImageMb !== c.maxImageMb) {
    setPrev(c);
    setMaxItems(String(c.maxItems));
    setMaxImageMb(String(c.maxImageMb));
  }

  async function doClear() {
    if (!window.confirm("고정하지 않은 클립보드 항목을 모두 삭제할까요?")) return;
    try {
      await clearClips();
      onNotice({ kind: "success", text: "클립보드 히스토리를 삭제했습니다." });
    } catch (e) {
      onNotice({ kind: "error", text: `삭제 실패: ${e}` });
    }
  }

  return (
    <section>
      <SettingsCard icon="clipboard" title="팝업 단축키">
        <HotkeyInput
          value={c.hotkey}
          error={maps.errors["clipboard.hotkey"]}
          failure={maps.failures["clipboard.hotkey"]}
          onChange={(hotkey) => set({ hotkey })}
        />
      </SettingsCard>
      <SettingsCard title="최대 개수" description="1–10000개. 넘으면 오래된 항목부터 지웁니다(고정 제외)">
        <input
          type="number"
          min={1}
          max={10000}
          value={maxItems}
          onChange={(e) => setMaxItems(e.target.value)}
          onBlur={() => {
            const v = clamp(Number(maxItems), 1, 10000);
            setMaxItems(String(v));
            set({ maxItems: v });
          }}
        />
      </SettingsCard>
      <SettingsCard title="이미지 최대 크기" description="1–100MB. 넘는 이미지는 기록하지 않습니다">
        <input
          type="number"
          min={1}
          max={100}
          value={maxImageMb}
          onChange={(e) => setMaxImageMb(e.target.value)}
          onBlur={() => {
            const v = clamp(Number(maxImageMb), 1, 100);
            setMaxImageMb(String(v));
            set({ maxImageMb: v });
          }}
        />
      </SettingsCard>
      <SettingsCard icon="delete" title="히스토리 비우기" description="고정한 항목은 남깁니다">
        <button className="danger" onClick={doClear}>
          전체 삭제
        </button>
      </SettingsCard>
    </section>
  );
}
