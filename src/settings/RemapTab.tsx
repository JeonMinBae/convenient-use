import { useState } from "react";
import type { KeyCombo, RemapRule, Settings } from "../api";
import { HotkeyInput } from "../components/HotkeyInput";
import { Icon } from "../components/Icon";
import { InfoBar } from "../components/InfoBar";
import { KeyField } from "../components/Keycap";
import { SettingsGroup } from "../components/SettingsCard";
import { ToggleSwitch } from "../components/ToggleSwitch";
import { keyLabel } from "../lib/accelerator";
import type { FieldMaps } from "./fields";

interface Props {
  draft: Settings;
  onChange: (s: Settings) => void;
  maps: FieldMaps;
}

const MODS = ["Ctrl", "Alt", "Shift", "Super"] as const;
const EMPTY: KeyCombo = { mods: [], key: null };
const TYPE_LABELS: Record<RemapRule["type"], string> = {
  simple: "단일 키",
  tapHold: "탭 / 홀드",
  layer: "레이어",
  macro: "매크로",
};

function newRule(type: RemapRule["type"]): RemapRule {
  switch (type) {
    case "simple": return { type, from: "", to: { ...EMPTY } };
    case "tapHold": return { type, key: "", tap: { ...EMPTY }, hold: { ...EMPTY } };
    case "layer": return { type, trigger: "", tap: null, mappings: [] };
    case "macro": return { type, trigger: "", steps: [] };
  }
}

/** 물리 키(KeyboardEvent.code)를 녹화한다. 수정자 단독 키도 유효하다 (예: CapsLock, AltRight). */
function KeyInput({ value, onChange, error }: { value: string; onChange: (v: string) => void; error?: string }) {
  const [recording, setRecording] = useState(false);
  return (
    <span className="hotkey-wrap">
      <KeyField
        keys={value ? [keyLabel(value)] : []}
        recording={recording}
        placeholder="클릭 후 키를 누르세요"
        error={error}
        title={error}
        onFocus={() => setRecording(true)}
        onBlur={() => setRecording(false)}
        onKeyDown={(e) => {
          e.preventDefault();
          onChange(e.nativeEvent.code);
        }}
      />
      {error && <span className="field-error">{error}</span>}
    </span>
  );
}

function ComboEditor({ value, onChange, error }: { value: KeyCombo; onChange: (c: KeyCombo) => void; error?: string }) {
  const toggle = (m: string) =>
    onChange({ ...value, mods: MODS.filter((x) => (x === m ? !value.mods.includes(m) : value.mods.includes(x))) });
  return (
    <span className="hotkey-wrap">
      {MODS.map((m) => (
        <label key={m}>
          <input type="checkbox" checked={value.mods.includes(m)} onChange={() => toggle(m)} />
          {m}
        </label>
      ))}
      <KeyInput value={value.key ?? ""} onChange={(key) => onChange({ ...value, key })} />
      {value.key && (
        <button className="subtle" onClick={() => onChange({ ...value, key: null })}>
          키 지우기
        </button>
      )}
      {error && <span className="field-error">{error}</span>}
    </span>
  );
}

export function RemapTab({ draft, onChange, maps }: Props) {
  const { rules, excludedApps } = draft.remap;
  const setRemap = (patch: Partial<Settings["remap"]>) => onChange({ ...draft, remap: { ...draft.remap, ...patch } });
  const setRule = (i: number, r: RemapRule) => setRemap({ rules: rules.map((x, k) => (k === i ? r : x)) });
  const err = (i: number, name: string) => maps.errors[`remap.rules.${i}.${name}`];

  const delBtn = (onClick: () => void) => (
    <button className="subtle icon-btn" title="삭제" aria-label="삭제" onClick={onClick}>
      <Icon name="delete" />
    </button>
  );
  const row = (label: string, node: React.ReactNode) => (
    <div className="row">
      <span className="label">{label}</span>
      {node}
    </div>
  );

  return (
    <section>
      <InfoBar
        notice={{
          kind: "info",
          text: "이 탭에서 키를 녹화할 때도 리매핑이 동작 중이므로, 이미 바꾼 키는 바뀐 값으로 입력될 수 있습니다.",
        }}
      />
      {rules.length === 0 && (
        <div className="empty-state">
          <Icon name="keyboard" size={32} />
          <span>등록된 규칙이 없습니다.</span>
        </div>
      )}
      {rules.map((r, i) => (
        <div className="card" key={i}>
          <div className="card-header">
            <strong>{TYPE_LABELS[r.type]}</strong>
            {delBtn(() => setRemap({ rules: rules.filter((_, k) => k !== i) }))}
          </div>
          {r.type === "simple" && (
            <>
              {row("원래 키", <KeyInput value={r.from} error={err(i, "from")} onChange={(from) => setRule(i, { ...r, from })} />)}
              {row("바꿀 키", <ComboEditor value={r.to} error={err(i, "to")} onChange={(to) => setRule(i, { ...r, to })} />)}
            </>
          )}
          {r.type === "tapHold" && (
            <>
              {row("키", <KeyInput value={r.key} error={err(i, "key")} onChange={(key) => setRule(i, { ...r, key })} />)}
              {row("탭 (200ms 이내)", <ComboEditor value={r.tap} error={err(i, "tap")} onChange={(tap) => setRule(i, { ...r, tap })} />)}
              {row("홀드", <ComboEditor value={r.hold} error={err(i, "hold")} onChange={(hold) => setRule(i, { ...r, hold })} />)}
            </>
          )}
          {r.type === "layer" && (
            <>
              {row("트리거 키", <KeyInput value={r.trigger} error={err(i, "trigger")} onChange={(trigger) => setRule(i, { ...r, trigger })} />)}
              {row(
                "탭 동작",
                <>
                  <ToggleSwitch
                    label="탭 동작"
                    checked={r.tap !== null}
                    onChange={(on) => setRule(i, { ...r, tap: on ? { ...EMPTY } : null })}
                  />
                  {r.tap && <ComboEditor value={r.tap} error={err(i, "tap")} onChange={(tap) => setRule(i, { ...r, tap })} />}
                </>,
              )}
              {r.mappings.map((m, j) => (
                <div className="row" key={j}>
                  <KeyInput
                    value={m.from}
                    error={err(i, `mappings.${j}.from`)}
                    onChange={(from) => setRule(i, { ...r, mappings: r.mappings.map((x, k) => (k === j ? { ...x, from } : x)) })}
                  />
                  <span>→</span>
                  <ComboEditor
                    value={m.to}
                    error={err(i, `mappings.${j}.to`)}
                    onChange={(to) => setRule(i, { ...r, mappings: r.mappings.map((x, k) => (k === j ? { ...x, to } : x)) })}
                  />
                  {delBtn(() => setRule(i, { ...r, mappings: r.mappings.filter((_, k) => k !== j) }))}
                </div>
              ))}
              <button onClick={() => setRule(i, { ...r, mappings: [...r.mappings, { from: "", to: { ...EMPTY } }] })}>
                <Icon name="add" /> 매핑 추가
              </button>
            </>
          )}
          {r.type === "macro" && (
            <>
              {row(
                "단축키",
                <HotkeyInput value={r.trigger} error={err(i, "trigger")} onChange={(trigger) => setRule(i, { ...r, trigger })} />,
              )}
              {err(i, "steps") && <p className="field-error">{err(i, "steps")}</p>}
              {r.steps.map((c, j) => (
                <div className="row" key={j}>
                  <span className="label">{j + 1}단계</span>
                  <ComboEditor
                    value={c}
                    error={err(i, `steps.${j}`)}
                    onChange={(n) => setRule(i, { ...r, steps: r.steps.map((x, k) => (k === j ? n : x)) })}
                  />
                  {delBtn(() => setRule(i, { ...r, steps: r.steps.filter((_, k) => k !== j) }))}
                </div>
              ))}
              <button onClick={() => setRule(i, { ...r, steps: [...r.steps, { ...EMPTY }] })}>
                <Icon name="add" /> 단계 추가
              </button>
            </>
          )}
        </div>
      ))}
      <div className="button-row">
        {(Object.keys(TYPE_LABELS) as RemapRule["type"][]).map((t) => (
          <button key={t} onClick={() => setRemap({ rules: [...rules, newRule(t)] })}>
            <Icon name="add" /> {TYPE_LABELS[t]} 추가
          </button>
        ))}
      </div>

      <SettingsGroup title="제외 앱">
        <InfoBar
          notice={{
            kind: "info",
            text: "포그라운드 창이 이 앱이면 리매핑을 하지 않습니다. 실행 파일 이름을 입력하세요 (예: notepad.exe).",
          }}
        />
        {excludedApps.map((a, i) => (
          <div className="card" key={i}>
            <div className="row">
              <input
                className="grow"
                placeholder="notepad.exe"
                value={a}
                onChange={(e) => setRemap({ excludedApps: excludedApps.map((x, k) => (k === i ? e.target.value : x)) })}
              />
              {delBtn(() => setRemap({ excludedApps: excludedApps.filter((_, k) => k !== i) }))}
            </div>
          </div>
        ))}
        <button onClick={() => setRemap({ excludedApps: [...excludedApps, ""] })}>
          <Icon name="add" /> 제외 앱 추가
        </button>
      </SettingsGroup>
    </section>
  );
}
