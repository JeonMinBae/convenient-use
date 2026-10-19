import type { Settings, Snippet } from "../api";
import { Icon } from "../components/Icon";
import { InfoBar } from "../components/InfoBar";
import type { FieldMaps } from "./fields";

interface Props {
  draft: Settings;
  onChange: (s: Settings) => void;
  maps: FieldMaps;
}

export function SnippetTab({ draft, onChange, maps }: Props) {
  const items = draft.snippets;
  const update = (id: string, patch: Partial<Snippet>) =>
    onChange({ ...draft, snippets: items.map((i) => (i.id === id ? { ...i, ...patch } : i)) });
  const remove = (id: string) => onChange({ ...draft, snippets: items.filter((i) => i.id !== id) });
  const add = () => onChange({ ...draft, snippets: [...items, { id: crypto.randomUUID(), keyword: "", text: "" }] });

  return (
    <section>
      <InfoBar
        notice={{
          kind: "info",
          text: "키워드를 입력하는 즉시 텍스트로 바뀝니다. 영문·숫자·기호 2~32자, 한글 입력 모드에서는 동작하지 않습니다.",
        }}
      />
      {items.length === 0 && (
        <div className="empty-state">
          <Icon name="textExpand" size={32} />
          <span>등록된 스니펫이 없습니다.</span>
        </div>
      )}
      {items.map((s) => {
        const error = maps.errors[`snippets.${s.id}.keyword`];
        return (
          <div className="card" key={s.id}>
            <div className="row">
              <input
                className="grow"
                placeholder=";addr"
                value={s.keyword}
                aria-invalid={!!error}
                onChange={(e) => update(s.id, { keyword: e.target.value })}
              />
              <button className="subtle icon-btn" title="삭제" aria-label="삭제" onClick={() => remove(s.id)}>
                <Icon name="delete" />
              </button>
            </div>
            {error && <p className="field-error">{error}</p>}
            <textarea
              className="grow"
              style={{ width: "100%" }}
              rows={3}
              placeholder="치환할 텍스트 (여러 줄 가능)"
              value={s.text}
              onChange={(e) => update(s.id, { text: e.target.value })}
            />
          </div>
        );
      })}
      <button onClick={add}>
        <Icon name="add" /> 스니펫 추가
      </button>
    </section>
  );
}
