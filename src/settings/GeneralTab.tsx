import { open, save } from "@tauri-apps/plugin-dialog";
import { type FeatureToggles, type Settings, type ValidationError, exportSettings, importSettings } from "../api";

const FEATURES: [keyof FeatureToggles, string][] = [
  ["launch", "실행 단축키"],
  ["window", "창 조절"],
  ["clipboard", "클립보드 히스토리"],
  ["snippet", "스니펫"],
  ["remap", "키 리매핑"],
];

interface Props {
  draft: Settings;
  onChange: (s: Settings) => void;
  onImported: (s: Settings) => void;
  onMessage: (m: string) => void;
}

export function GeneralTab({ draft, onChange, onImported, onMessage }: Props) {
  const g = draft.general;

  async function doExport() {
    const path = await save({ filters: [{ name: "JSON", extensions: ["json"] }], defaultPath: "convenient-use-settings.json" });
    if (!path) return;
    try {
      // 백엔드에 저장된 설정을 내보낸다 (저장하지 않은 편집은 포함되지 않음)
      await exportSettings(path);
      onMessage("설정을 내보냈습니다.");
    } catch (e) {
      onMessage(`내보내기 실패: ${e}`);
    }
  }

  async function doImport() {
    const path = await open({ multiple: false, filters: [{ name: "JSON", extensions: ["json"] }] });
    if (typeof path !== "string") return;
    try {
      onImported(await importSettings(path));
      onMessage("설정을 가져왔습니다.");
    } catch (e) {
      // 검증 실패는 ValidationError[] 로 온다
      onMessage(
        Array.isArray(e)
          ? `가져오기 실패: ${(e as ValidationError[]).map((x) => x.message).join(" / ")}`
          : `가져오기 실패: ${e}`,
      );
    }
  }

  return (
    <section>
      <label className="row">
        <input
          type="checkbox"
          checked={g.autostart}
          onChange={(e) => onChange({ ...draft, general: { ...g, autostart: e.target.checked } })}
        />
        로그인 시 자동 실행
      </label>
      <h3>기능</h3>
      {FEATURES.map(([key, label]) => (
        <label className="row" key={key}>
          <input
            type="checkbox"
            checked={g.features[key]}
            onChange={(e) => onChange({ ...draft, general: { ...g, features: { ...g.features, [key]: e.target.checked } } })}
          />
          {label}
        </label>
      ))}
      <h3>설정 파일</h3>
      <div className="row">
        <button onClick={doExport}>내보내기</button>
        <button onClick={doImport}>가져오기</button>
      </div>
    </section>
  );
}
