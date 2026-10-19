import { getCurrentWindow } from "@tauri-apps/api/window";
import appIcon from "../../src-tauri/icons/32x32.png";
import { Icon } from "./Icon";

// 버튼에는 drag-region 을 두지 않는다(두면 클릭이 드래그로 먹힌다)
export function TitleBar() {
  const win = getCurrentWindow();
  return (
    <div className="titlebar" data-tauri-drag-region>
      <img className="titlebar-icon" src={appIcon} alt="" data-tauri-drag-region />
      <span className="titlebar-title" data-tauri-drag-region>Convenient Use</span>
      <button className="caption-btn" title="최소화" onClick={() => void win.minimize()}>
        <Icon name="subtract" />
      </button>
      {/* lib.rs 가 CloseRequested 를 hide 로 바꾼다 */}
      <button className="caption-btn close" title="닫기" onClick={() => void win.close()}>
        <Icon name="dismiss" />
      </button>
    </div>
  );
}
