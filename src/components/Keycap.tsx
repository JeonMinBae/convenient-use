export const Keycap = ({ label }: { label: string }) => <kbd className="keycap">{label}</kbd>;

interface KeyFieldProps {
  keys: string[];
  recording: boolean;
  placeholder: string;
  error?: string;
  title?: string;
  onFocus: () => void;
  onBlur: () => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLDivElement>) => void;
}

/** 키 녹화 필드. input 이 아니라 div 라서 키캡을 넣을 수 있고, 버튼과 달리 Enter/Space 클릭 동작이 없다 */
export function KeyField({ keys, recording, placeholder, error, title, ...handlers }: KeyFieldProps) {
  const cls = ["keyfield", recording && "recording", error && "error"].filter(Boolean).join(" ");
  return (
    <div role="button" tabIndex={0} className={cls} title={title} aria-invalid={!!error} {...handlers}>
      {recording ? (
        <span className="keyfield-hint">키를 누르세요…</span>
      ) : keys.length ? (
        keys.map((k, i) => <Keycap key={i} label={k} />)
      ) : (
        <span className="keyfield-hint">{placeholder}</span>
      )}
    </div>
  );
}
