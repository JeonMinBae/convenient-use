import { open, save } from "@tauri-apps/plugin-dialog";
import { type FeatureToggles, type Settings, type ValidationError, exportSettings, importSettings } from "../api";
import { Icon, type IconName } from "../components/Icon";
import type { Notice } from "../components/InfoBar";
import { SettingsCard, SettingsGroup } from "../components/SettingsCard";
import { ToggleSwitch } from "../components/ToggleSwitch";

// 아이콘은 사이드바와 같게 유지한다
const FEATURES: [keyof FeatureToggles, string, IconName, string][] = [
  ["launch", "실행 단축키", "rocket", "단축키로 앱이나 웹사이트를 엽니다"],
  ["window", "창 조절", "window", "단축키로 창을 화면 절반·1/3 등에 배치합니다"],
  ["clipboard", "클립보드 히스토리", "clipboard", "복사한 텍스트·이미지·파일을 기록하고 다시 붙여넣습니다"],
  ["snippet", "스니펫", "textExpand", "키워드를 입력하면 지정한 텍스트로 바꿉니다"],
  ["remap", "키 리매핑", "keyboard", "키를 다른 키·조합·매크로로 바꿉니다"],
];

interface Props {
  draft: Settings;
  onChange: (s: Settings) => void;
  onImported: (s: Settings) => void;
  onNotice: (n: Notice) => void;
}

export function GeneralTab({ draft, onChange, onImported, onNotice }: Props) {
  const g = draft.general;

  async function doExport() {
    const path = await save({ filters: [{ name: "JSON", extensions: ["json"] }], defaultPath: "convenient-use-settings.json" });
    if (!path) return;
    try {
      // 백엔드에 저장된 설정을 내보낸다 (저장하지 않은 편집은 포함되지 않음)
      await exportSettings(path);
      onNotice({ kind: "success", text: "설정을 내보냈습니다." });
    } catch (e) {
      onNotice({ kind: "error", text: `내보내기 실패: ${e}` });
    }
  }

  async function doImport() {
    const path = await open({ multiple: false, filters: [{ name: "JSON", extensions: ["json"] }] });
    if (typeof path !== "string") return;
    try {
      onImported(await importSettings(path));
      onNotice({ kind: "success", text: "설정을 가져왔습니다." });
    } catch (e) {
      // 검증 실패는 ValidationError[] 로 온다
      onNotice({
        kind: "error",
        text: Array.isArray(e)
          ? `가져오기 실패: ${(e as ValidationError[]).map((x) => x.message).join(" / ")}`
          : `가져오기 실패: ${e}`,
      });
    }
  }

  return (
    <section>
      <SettingsCard icon="power" title="로그인 시 자동 실행" description="Windows에 로그인하면 트레이에서 자동으로 시작합니다">
        <ToggleSwitch
          label="로그인 시 자동 실행"
          checked={g.autostart}
          onChange={(autostart) => onChange({ ...draft, general: { ...g, autostart } })}
        />
      </SettingsCard>
      <SettingsGroup title="기능">
        {FEATURES.map(([key, label, icon, description]) => (
          <SettingsCard key={key} icon={icon} title={label} description={description}>
            <ToggleSwitch
              label={label}
              checked={g.features[key]}
              onChange={(on) => onChange({ ...draft, general: { ...g, features: { ...g.features, [key]: on } } })}
            />
          </SettingsCard>
        ))}
      </SettingsGroup>
      <SettingsGroup title="설정 파일">
        <SettingsCard title="내보내기·가져오기" description="저장된 설정을 JSON 파일로 백업하거나 복원합니다">
          <button onClick={doExport}>
            <Icon name="arrowExportUp" /> 내보내기
          </button>
          <button onClick={doImport}>
            <Icon name="arrowImport" /> 가져오기
          </button>
        </SettingsCard>
      </SettingsGroup>
    </section>
  );
}
