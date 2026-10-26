import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useState } from "react";
import { type HotkeyFailure, type Settings, type ValidationError, getHotkeyFailures, getSettings, saveSettings } from "../api";
import { Icon, type IconName } from "../components/Icon";
import { InfoBar, type Notice } from "../components/InfoBar";
import { TitleBar } from "../components/TitleBar";
import { ClipboardTab } from "./ClipboardTab";
import { GeneralTab } from "./GeneralTab";
import { LaunchTab } from "./LaunchTab";
import { RemapTab } from "./RemapTab";
import { SnippetTab } from "./SnippetTab";
import { WindowTab } from "./WindowTab";
import { type SettingsPage, buildFieldMaps, isDirty, pagesWithIssues } from "./fields";

const TABS: [SettingsPage, string, IconName][] = [
  ["general", "일반", "settings"],
  ["launch", "실행 단축키", "rocket"],
  ["window", "창 조절", "window"],
  ["clipboard", "클립보드", "clipboard"],
  ["snippet", "스니펫", "textExpand"],
  ["remap", "키 리매핑", "keyboard"],
];

export default function SettingsApp() {
  const [tab, setTab] = useState<SettingsPage>("general");
  const [draft, setDraft] = useState<Settings | null>(null);
  // 마지막으로 저장·로드된 설정. draft 와 객체를 공유하지 않는다
  const [baseline, setBaseline] = useState<Settings | null>(null);
  const [errors, setErrors] = useState<ValidationError[]>([]);
  const [failures, setFailures] = useState<HotkeyFailure[]>([]);
  const [notice, setNotice] = useState<Notice | null>(null);

  const load = useCallback(async () => {
    const s = await getSettings();
    setDraft(s);
    setBaseline(structuredClone(s));
    setFailures(await getHotkeyFailures().catch(() => []));
    setErrors([]);
  }, []);

  const [loadFailed, setLoadFailed] = useState(false);

  const tryLoad = useCallback(() => {
    load()
      .then(() => {
        setLoadFailed(false);
        setNotice(null);
      })
      .catch((e) => {
        setLoadFailed(true);
        setNotice({ kind: "error", text: `설정을 불러오지 못했습니다: ${e}` });
      });
  }, [load]);

  useEffect(tryLoad, [tryLoad]);

  // 창은 숨겨질 뿐 다시 마운트되지 않으므로, 불러오기에 실패했다면 창이 다시 포커스될 때 재시도한다
  useEffect(() => {
    if (!loadFailed) return;
    const unlisten = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) tryLoad();
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [loadFailed, tryLoad]);

  if (!draft) {
    return (
      <div className="settings">
        <TitleBar />
        <div className="settings-loading">
          {notice ? (
            <InfoBar notice={notice} onClose={() => setNotice(null)} />
          ) : (
            // 실패 알림을 닫은 뒤에도 로딩 중으로 보이지 않게 한다
            <p className="empty">{loadFailed ? "설정을 불러오지 못했습니다." : "불러오는 중…"}</p>
          )}
        </div>
      </div>
    );
  }

  async function save() {
    const saving = draft!;
    try {
      const r = await saveSettings(saving);
      setErrors([]);
      setFailures(r.hotkeyFailures);
      setBaseline(structuredClone(saving));
      setNotice(
        r.hotkeyFailures.length
          ? { kind: "warning", text: "저장했지만 일부 단축키를 등록하지 못했습니다." }
          : { kind: "success", text: "저장했습니다." },
      );
    } catch (e) {
      // 실패하면 draft 는 그대로 둔다
      if (Array.isArray(e)) {
        const list = e as ValidationError[];
        setErrors(list);
        // field 가 빈 오류(저장소 오류 등)는 필드에 붙일 곳이 없어 메시지로 보여 준다
        setNotice({ kind: "error", text: list.find((x) => !x.field)?.message ?? "입력을 확인하세요." });
      } else {
        setNotice({ kind: "error", text: `저장 실패: ${e}` });
      }
    }
  }

  function revert() {
    setDraft(structuredClone(baseline!));
    setErrors([]);
    setNotice(null);
  }

  const maps = buildFieldMaps(errors, failures);
  const issuePages = pagesWithIssues(maps);
  const dirty = isDirty(draft, baseline);
  const onChange = (s: Settings) => setDraft(s);

  return (
    <div className="settings">
      <TitleBar />
      <div className="settings-body">
        <nav className="sidebar">
          {TABS.map(([key, label, icon]) => (
            <button key={key} className={key === tab ? "sidebar-item active" : "sidebar-item"} onClick={() => setTab(key)}>
              <Icon name={icon} size={20} />
              <span className="sidebar-label">{label}</span>
              {issuePages.has(key) && <span className="sidebar-dot" title="확인이 필요한 항목이 있습니다" />}
            </button>
          ))}
        </nav>
        <div className="main-col">
          <main className="content">
            <h1 className="page-title">{TABS.find(([k]) => k === tab)![1]}</h1>
            {notice && <InfoBar notice={notice} onClose={() => setNotice(null)} />}
            {tab === "general" && (
              <GeneralTab
                draft={draft}
                onChange={onChange}
                onNotice={setNotice}
                onImported={(s) => {
                  // 가져오기는 백엔드에 즉시 반영되므로 받은 설정이 곧 기준선이다. 단축키 등록 실패만 다시 조회한다
                  setDraft(s);
                  setBaseline(structuredClone(s));
                  setErrors([]);
                  getHotkeyFailures().then(setFailures, () => setFailures([]));
                }}
              />
            )}
            {tab === "launch" && <LaunchTab draft={draft} onChange={onChange} maps={maps} />}
            {tab === "window" && <WindowTab draft={draft} onChange={onChange} maps={maps} />}
            {tab === "clipboard" && <ClipboardTab draft={draft} onChange={onChange} maps={maps} onNotice={setNotice} />}
            {tab === "snippet" && <SnippetTab draft={draft} onChange={onChange} maps={maps} />}
            {tab === "remap" && <RemapTab draft={draft} onChange={onChange} maps={maps} />}
          </main>
          {dirty && (
            <div className="savebar">
              <span className="savebar-text">저장되지 않은 변경 사항</span>
              <button onClick={revert}>
                <Icon name="arrowUndo" /> 되돌리기
              </button>
              <button className="primary" onClick={save}>
                저장
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
