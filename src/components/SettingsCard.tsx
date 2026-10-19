import type { ReactNode } from "react";
import { Icon, type IconName } from "./Icon";

export function SettingsCard({ icon, title, description, children, className }: {
  icon?: IconName; title: string; description?: string; children?: ReactNode; className?: string;
}) {
  return (
    <div className={className ? `settings-card ${className}` : "settings-card"}>
      {icon && <Icon name={icon} size={20} className="settings-card-icon" />}
      <div className="settings-card-text">
        <div className="settings-card-title">{title}</div>
        {description && <div className="settings-card-desc">{description}</div>}
      </div>
      <div className="settings-card-control">{children}</div>
    </div>
  );
}

export function SettingsGroup({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="settings-group">
      <h2 className="settings-group-title">{title}</h2>
      {children}
    </section>
  );
}
