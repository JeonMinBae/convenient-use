// 체크박스는 시각적으로 숨기되 DOM에 남겨 키보드(Space)·접근성 기본 동작을 유지한다
export function ToggleSwitch({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label?: string }) {
  return (
    <label className="toggle">
      <span className="toggle-text">{checked ? "켬" : "끔"}</span>
      <input type="checkbox" role="switch" aria-label={label} checked={checked} onChange={(e) => onChange(e.target.checked)} />
      <span className="toggle-track" aria-hidden="true"><span className="toggle-thumb" /></span>
    </label>
  );
}
