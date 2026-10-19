import type { LayoutCommand, Settings } from "../api";
import { HotkeyInput } from "../components/HotkeyInput";
import { SettingsCard, SettingsGroup } from "../components/SettingsCard";
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

// 마지막 "기타" 그룹은 목록에 없는 명령도 받는다
const GROUPS: [string, LayoutCommand[]][] = [
  ["반쪽", ["leftHalf", "rightHalf", "topHalf", "bottomHalf"]],
  ["1/4", ["topLeft", "topRight", "bottomLeft", "bottomRight"]],
  ["1/3 · 2/3", ["firstThird", "centerThird", "lastThird", "firstTwoThirds", "lastTwoThirds"]],
];
const GROUPED = new Set<string>(GROUPS.flatMap(([, cmds]) => cmds));

interface Props {
  draft: Settings;
  onChange: (s: Settings) => void;
  maps: FieldMaps;
}

export function WindowTab({ draft, onChange, maps }: Props) {
  const renderCard = (w: Settings["windowHotkeys"][number], i: number) => {
    const field = `windowHotkeys.${w.command}`;
    return (
      <SettingsCard key={w.command} title={LAYOUT_LABELS[w.command] ?? w.command}>
        <HotkeyInput
          value={w.hotkey}
          error={maps.errors[field]}
          failure={maps.failures[field]}
          onChange={(hotkey) =>
            onChange({ ...draft, windowHotkeys: draft.windowHotkeys.map((x, j) => (j === i ? { ...x, hotkey } : x)) })
          }
        />
      </SettingsCard>
    );
  };
  const indexed = draft.windowHotkeys.map((w, i) => [w, i] as const);
  const groups: [string, (readonly [Settings["windowHotkeys"][number], number])[]][] = [
    ...GROUPS.map(([title, cmds]): [string, (readonly [Settings["windowHotkeys"][number], number])[]] => [
      title,
      indexed.filter(([w]) => cmds.includes(w.command)),
    ]),
    ["기타", indexed.filter(([w]) => !GROUPED.has(w.command))],
  ];

  return (
    <section>
      {groups.map(
        ([title, list]) =>
          list.length > 0 && (
            <SettingsGroup key={title} title={title}>
              {list.map(([w, i]) => renderCard(w, i))}
            </SettingsGroup>
          ),
      )}
    </section>
  );
}
