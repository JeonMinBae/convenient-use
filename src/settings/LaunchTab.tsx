import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { type InstalledApp, type LaunchItem, type OsPath, type Settings, listInstalledApps } from "../api";
import { HotkeyInput } from "../components/HotkeyInput";
import { Icon } from "../components/Icon";
import { type FieldMaps, isWindows, osKey } from "./fields";

interface Props {
  draft: Settings;
  onChange: (s: Settings) => void;
  maps: FieldMaps;
}

// 현재 OS 키만 바꾸고 다른 OS 값은 보존한다. 빈 값이면 현재 OS 키만 비운다.
const withPath = (p: OsPath, v: string | null): OsPath => ({ ...p, [osKey]: v });
const currentPath = (p: OsPath) => p[osKey] ?? "";

const baseName = (p: string) => (p.split(/[\\/]/).pop() ?? p).replace(/\.(exe|lnk|app)$/i, "");

async function pickExecutable(): Promise<string | null> {
  // macOS 앱은 .app 번들(폴더)이지만 열기 대화상자는 파일처럼 고르게 해 준다
  const filter = isWindows ? { name: "실행 파일", extensions: ["exe", "lnk"] } : { name: "응용 프로그램", extensions: ["app"] };
  const r = await open({ multiple: false, filters: [filter], defaultPath: isWindows ? undefined : "/Applications" });
  return typeof r === "string" ? r : null;
}

export function LaunchTab({ draft, onChange, maps }: Props) {
  const [apps, setApps] = useState<InstalledApp[]>([]);
  useEffect(() => {
    listInstalledApps().then(setApps).catch(() => setApps([]));
  }, []);

  const items = draft.launchItems;
  const update = (id: string, f: (i: LaunchItem) => LaunchItem) =>
    onChange({ ...draft, launchItems: items.map((i) => (i.id === id ? f(i) : i)) });
  const remove = (id: string) => onChange({ ...draft, launchItems: items.filter((i) => i.id !== id) });
  const add = () =>
    onChange({
      ...draft,
      launchItems: [...items, { id: crypto.randomUUID(), name: "", hotkey: "", target: { type: "app", path: {} } }],
    });

  return (
    <section>
      {items.length === 0 && (
        <div className="empty-state">
          <Icon name="rocket" size={32} />
          <span>등록된 항목이 없습니다.</span>
        </div>
      )}
      {items.map((item) => (
        <LaunchRow
          key={item.id}
          item={item}
          apps={apps}
          maps={maps}
          onChange={(f) => update(item.id, f)}
          onRemove={() => remove(item.id)}
        />
      ))}
      <button onClick={add}>
        <Icon name="add" /> 항목 추가
      </button>
    </section>
  );
}

interface RowProps {
  item: LaunchItem;
  apps: InstalledApp[];
  maps: FieldMaps;
  onChange: (f: (i: LaunchItem) => LaunchItem) => void;
  onRemove: () => void;
}

function LaunchRow({ item, apps, maps, onChange, onRemove }: RowProps) {
  const [filter, setFilter] = useState("");
  const field = `launchItems.${item.id}.hotkey`;
  const t = item.target;

  const filtered = useMemo(() => {
    const q = filter.trim().toLowerCase();
    return (q ? apps.filter((a) => a.name.toLowerCase().includes(q)) : apps).slice(0, 200);
  }, [apps, filter]);

  const setApp = (path: string, name?: string) =>
    onChange((i) => ({
      ...i,
      name: i.name || name || baseName(path),
      target: { type: "app", path: withPath(i.target.type === "app" ? i.target.path : {}, path) },
    }));

  return (
    <div className="card">
      <div className="row">
        <input
          className="grow"
          placeholder="이름"
          value={item.name}
          onChange={(e) => onChange((i) => ({ ...i, name: e.target.value }))}
        />
        <Icon name={t.type === "app" ? "apps" : "globe"} size={20} />
        <select
          value={t.type}
          onChange={(e) =>
            onChange((i) => ({
              ...i,
              target: e.target.value === "app" ? { type: "app", path: {} } : { type: "url", url: "", browser: {} },
            }))
          }
        >
          <option value="app">앱</option>
          <option value="url">URL</option>
        </select>
        <HotkeyInput
          value={item.hotkey}
          error={maps.errors[field]}
          failure={maps.failures[field]}
          onChange={(hotkey) => onChange((i) => ({ ...i, hotkey }))}
        />
        <button className="subtle icon-btn" title="삭제" aria-label="삭제" onClick={onRemove}>
          <Icon name="delete" />
        </button>
      </div>

      {t.type === "app" ? (
        <div className="row">
          <input placeholder="설치된 앱 검색" value={filter} onChange={(e) => setFilter(e.target.value)} />
          <select
            value=""
            onChange={(e) => {
              const a = apps.find((x) => x.path === e.target.value);
              if (a) setApp(a.path, a.name);
            }}
          >
            <option value="">설치된 앱에서 선택… ({filtered.length})</option>
            {filtered.map((a) => (
              <option key={a.path} value={a.path}>
                {a.name}
              </option>
            ))}
          </select>
          <button
            className="icon-btn"
            title="파일 선택"
            aria-label="파일 선택"
            onClick={async () => {
              const p = await pickExecutable();
              if (p) setApp(p);
            }}
          >
            <Icon name="folderOpen" />
          </button>
          <input
            className="grow"
            placeholder="경로"
            value={currentPath(t.path)}
            onChange={(e) => onChange((i) => ({ ...i, target: { type: "app", path: withPath(t.path, e.target.value) } }))}
          />
        </div>
      ) : (
        <div className="row">
          <input
            className="grow"
            placeholder="https://"
            value={t.url}
            onChange={(e) => onChange((i) => ({ ...i, target: { ...t, url: e.target.value } }))}
          />
          <input readOnly className="grow" placeholder="기본 브라우저" value={currentPath(t.browser)} />
          <button
            className="icon-btn"
            title="브라우저 선택"
            aria-label="브라우저 선택"
            onClick={async () => {
              const p = await pickExecutable();
              if (p) onChange((i) => ({ ...i, target: { ...t, browser: withPath(t.browser, p) } }));
            }}
          >
            <Icon name="folderOpen" />
          </button>
          <button className="subtle" onClick={() => onChange((i) => ({ ...i, target: { ...t, browser: withPath(t.browser, null) } }))}>
            지우기
          </button>
        </div>
      )}
    </div>
  );
}
