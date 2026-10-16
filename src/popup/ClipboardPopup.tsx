import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { type ClipItem, deleteClip, hideClipboardPopup, listClips, pasteClip, setPinned } from "../api";

function preview(c: ClipItem) {
  if (c.kind === "files") {
    const files = c.files ?? [];
    return files.length > 1 ? `${files[0]} 외 ${files.length - 1}개` : (files[0] ?? "");
  }
  // 첫 줄만 보여 준다
  return (c.text ?? "").split(/\r?\n/).find((l) => l.trim()) ?? "";
}

export default function ClipboardPopup() {
  const [query, setQuery] = useState("");
  const [clips, setClips] = useState<ClipItem[]>([]);
  const [sel, setSel] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const queryRef = useRef(query);
  queryRef.current = query;
  const list = useRef<HTMLUListElement>(null);

  const refetch = useCallback(async (q: string) => {
    try {
      const items = await listClips(q);
      setClips(items);
      setSel((i) => Math.min(i, Math.max(items.length - 1, 0)));
    } catch {
      setClips([]);
    }
  }, []);

  // 열릴 때마다 검색어·선택을 초기화하고 입력창에 포커스
  useEffect(() => {
    const un = listen("clipboard-popup-opened", () => {
      setQuery("");
      setSel(0);
      refetch("");
      input.current?.focus();
    });
    return () => {
      un.then((f) => f());
    };
  }, [refetch]);

  useEffect(() => {
    const t = setTimeout(() => {
      setSel(0);
      refetch(query);
    }, 150);
    return () => clearTimeout(t);
  }, [query, refetch]);

  useEffect(() => {
    list.current?.children[sel]?.scrollIntoView({ block: "nearest" });
  }, [sel]);

  function onKeyDown(e: React.KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSel((i) => Math.min(i + 1, clips.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSel((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter" && !e.nativeEvent.isComposing && clips[sel]) {
      pasteClip(clips[sel].id);
    } else if (e.key === "Escape") {
      hideClipboardPopup();
    }
  }

  return (
    <div className="popup" onKeyDown={onKeyDown}>
      <input
        ref={input}
        className="popup-search"
        autoFocus
        placeholder="클립보드 검색"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />
      <ul className="popup-list" ref={list}>
        {clips.map((c, i) => (
          <li
            key={c.id}
            className={i === sel ? "clip selected" : "clip"}
            onMouseMove={() => setSel(i)}
            onClick={() => pasteClip(c.id)}
          >
            {c.kind === "image" && c.thumbnail ? (
              <img className="clip-thumb" src={c.thumbnail} alt="" />
            ) : (
              <span className="clip-text">{preview(c)}</span>
            )}
            <button
              className={c.pinned ? "clip-btn on" : "clip-btn"}
              title={c.pinned ? "고정 해제" : "고정"}
              onClick={async (e) => {
                e.stopPropagation();
                await setPinned(c.id, !c.pinned);
                refetch(queryRef.current);
              }}
            >
              {c.pinned ? "★" : "☆"}
            </button>
            <button
              className="clip-btn"
              title="삭제"
              onClick={async (e) => {
                e.stopPropagation();
                await deleteClip(c.id);
                refetch(queryRef.current);
              }}
            >
              ✕
            </button>
          </li>
        ))}
        {clips.length === 0 && <li className="clip-empty">항목이 없습니다</li>}
      </ul>
    </div>
  );
}
