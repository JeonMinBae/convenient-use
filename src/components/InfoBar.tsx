import { Icon, type IconName } from "./Icon";

export type NoticeKind = "success" | "warning" | "error" | "info";
export interface Notice { kind: NoticeKind; text: string }

const ICONS: Record<NoticeKind, IconName> = {
  success: "checkmarkCircle", warning: "warning", error: "errorCircle", info: "info",
};

export function InfoBar({ notice, onClose }: { notice: Notice; onClose?: () => void }) {
  return (
    <div className={`infobar ${notice.kind}`} role={notice.kind === "error" ? "alert" : "status"}>
      <Icon name={ICONS[notice.kind]} className="infobar-icon" />
      <span className="infobar-text">{notice.text}</span>
      {onClose && (
        <button className="subtle icon-btn" title="닫기" onClick={onClose}>
          <Icon name="dismiss" />
        </button>
      )}
    </div>
  );
}
