import { useCallback, useEffect, useState } from "react";
import { type HotkeyFailure, type Settings, type ValidationError, getHotkeyFailures, getSettings, saveSettings } from "../api";
import { ClipboardTab } from "./ClipboardTab";
import { GeneralTab } from "./GeneralTab";
import { LaunchTab } from "./LaunchTab";
import { WindowTab } from "./WindowTab";
import { buildFieldMaps } from "./fields";

type Tab = "general" | "launch" | "window" | "clipboard";
const TABS: [Tab, string][] = [
  ["general", "일반"],
  ["launch", "실행 단축키"],
  ["window", "창 조절"],
  ["clipboard", "클립보드"],
];

export default function SettingsApp() {
  const [tab, setTab] = useState<Tab>("general");
  const [draft, setDraft] = useState<Settings | null>(null);
  const [errors, setErrors] = useState<ValidationError[]>([]);
  const [failures, setFailures] = useState<HotkeyFailure[]>([]);
  const [message, setMessage] = useState("");

  const load = useCallback(async () => {
    setDraft(await getSettings());
    setFailures(await getHotkeyFailures().catch(() => []));
    setErrors([]);
  }, []);

  useEffect(() => {
    load().catch((e) => setMessage(`설정을 불러오지 못했습니다: ${e}`));
  }, [load]);

  if (!draft) return <p className="empty">{message || "불러오는 중…"}</p>;

  async function save() {
    try {
      const r = await saveSettings(draft!);
      setErrors([]);
      setFailures(r.hotkeyFailures);
      setMessage(r.hotkeyFailures.length ? "저장했지만 일부 단축키를 등록하지 못했습니다." : "저장했습니다.");
    } catch (e) {
      // 실패하면 draft 는 그대로 둔다
      if (Array.isArray(e)) {
        const list = e as ValidationError[];
        setErrors(list);
        // field 가 빈 오류(저장소 오류 등)는 필드에 붙일 곳이 없어 메시지로 보여 준다
        setMessage(list.find((x) => !x.field)?.message ?? "입력을 확인하세요.");
      } else {
        setMessage(`저장 실패: ${e}`);
      }
    }
  }

  const maps = buildFieldMaps(errors, failures);
  const onChange = (s: Settings) => setDraft(s);

  return (
    <div className="settings">
      <nav className="tabs">
        {TABS.map(([key, label]) => (
          <button key={key} className={key === tab ? "tab active" : "tab"} onClick={() => setTab(key)}>
            {label}
          </button>
        ))}
      </nav>
      <main>
        {tab === "general" && (
          <GeneralTab
            draft={draft}
            onChange={onChange}
            onMessage={setMessage}
            onImported={() => {
              // 가져오기는 백엔드에 즉시 반영되므로 화면을 다시 불러온다
              load().catch((e) => setMessage(`새로고침 실패: ${e}`));
            }}
          />
        )}
        {tab === "launch" && <LaunchTab draft={draft} onChange={onChange} maps={maps} />}
        {tab === "window" && <WindowTab draft={draft} onChange={onChange} maps={maps} />}
        {tab === "clipboard" && <ClipboardTab draft={draft} onChange={onChange} maps={maps} onMessage={setMessage} />}
      </main>
      <footer className="footer">
        <span className="message">{message}</span>
        <button className="primary" onClick={save}>
          저장
        </button>
      </footer>
    </div>
  );
}
