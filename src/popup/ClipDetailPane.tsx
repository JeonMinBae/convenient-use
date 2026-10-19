import type { ClipDetail } from "../api";
import { Icon } from "../components/Icon";
import { formatTime, sizeLabel } from "../lib/clipDetail";

export default function ClipDetailPane({ detail }: { detail: ClipDetail | null }) {
  if (!detail)
    return (
      <aside className="clip-detail empty">
        <Icon name="clipboard" size={32} />
        <span>선택된 항목이 없습니다</span>
      </aside>
    );
  return (
    <aside className="clip-detail">
      <div className="clip-detail-body">
        {detail.kind === "image" ? (
          detail.image ? <img className="clip-detail-image" src={detail.image} alt="" /> : <p>이미지 파일을 찾을 수 없습니다</p>
        ) : detail.kind === "files" ? (
          <ul className="clip-detail-files">{(detail.files ?? []).map((f) => <li key={f}>{f}</li>)}</ul>
        ) : (
          <pre className="clip-detail-text">{detail.text}</pre>
        )}
      </div>
      <dl className="clip-detail-meta">
        <dt>처음 복사</dt><dd>{formatTime(detail.createdAt)}</dd>
        <dt>마지막 사용</dt><dd>{formatTime(detail.lastUsedAt)}</dd>
        <dt>크기</dt><dd>{sizeLabel(detail)}</dd>
      </dl>
    </aside>
  );
}
