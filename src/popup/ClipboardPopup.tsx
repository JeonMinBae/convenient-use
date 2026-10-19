import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  CLIP_PAGE_SIZE,
  type ClipDetail,
  type ClipItem,
  deleteClip,
  getClipDetail,
  hideClipboardPopup,
  listClips,
  pasteClip,
  setPinned,
} from "../api";
import ClipDetailPane from "./ClipDetailPane";

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
  const clipsRef = useRef(clips);
  clipsRef.current = clips;
  // 응답이 늦게 도착해 최신 검색 결과를 덮어쓰지 않도록 요청마다 번호를 매긴다
  const reqId = useRef(0);
  const exhausted = useRef(false);
  const loadingMore = useRef(false);
  const [detail, setDetail] = useState<ClipDetail | null>(null);
  // 목록을 다시 받을 때마다 증가. 선택 id 가 같아도 마지막 사용 시간 등이 바뀌었을 수 있다
  const [listVersion, setListVersion] = useState(0);
  const detailReq = useRef(0);
  const selectedId = clips[sel]?.id;

  const refetch = useCallback(async (q: string) => {
    const id = ++reqId.current;
    exhausted.current = false;
    loadingMore.current = false;
    try {
      const items = await listClips(q);
      if (id !== reqId.current) return;
      exhausted.current = items.length < CLIP_PAGE_SIZE;
      setClips(items);
      setListVersion((v) => v + 1);
      setSel((i) => Math.min(i, Math.max(items.length - 1, 0)));
    } catch {
      if (id === reqId.current) setClips([]);
    }
  }, []);

  const loadMore = useCallback(async () => {
    if (exhausted.current || loadingMore.current) return;
    loadingMore.current = true;
    const id = reqId.current;
    try {
      const items = await listClips(queryRef.current, clipsRef.current.length);
      if (id !== reqId.current) return;
      exhausted.current = items.length < CLIP_PAGE_SIZE;
      setClips((prev) => {
        const seen = new Set(prev.map((c) => c.id));
        return [...prev, ...items.filter((c) => !seen.has(c.id))];
      });
    } catch {
      exhausted.current = true;
    } finally {
      loadingMore.current = false;
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
    const id = ++detailReq.current;
    if (selectedId == null) {
      setDetail(null);
      return;
    }
    // 방향키를 누르고 있을 때 원본 이미지 요청이 몰리지 않게 잠깐 기다린다
    const t = setTimeout(async () => {
      try {
        const d = await getClipDetail(selectedId);
        if (id === detailReq.current) setDetail(d);
      } catch {
        if (id === detailReq.current) setDetail(null);
      }
    }, 80);
    return () => clearTimeout(t);
  }, [selectedId, listVersion]);

  useEffect(() => {
    list.current?.children[sel]?.scrollIntoView({ block: "nearest" });
  }, [sel]);

  function onKeyDown(e: React.KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSel((i) => Math.min(i + 1, clips.length - 1));
      if (sel >= clips.length - 1) loadMore();
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
      <div className="popup-body">
        <ul
          className="popup-list"
          ref={list}
          onScroll={(e) => {
            const el = e.currentTarget;
            if (el.scrollTop + el.clientHeight >= el.scrollHeight - 80) loadMore();
          }}
        >
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
        <ClipDetailPane detail={detail} />
      </div>
    </div>
  );
}
