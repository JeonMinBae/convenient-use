use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use sha2::{Digest, Sha256};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS clips (
  id INTEGER PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('text','image','files')),
  text TEXT,                 -- text: 본문, files: 경로를 \\n 으로 연결, image: NULL
  image_file TEXT,           -- image: uuid (확장자 제외)
  hash TEXT NOT NULL UNIQUE,
  pinned INTEGER NOT NULL DEFAULT 0,
  last_used_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_clips_last_used ON clips(last_used_at DESC);";

pub enum ClipContent {
    Text(String),
    Files(Vec<String>),
    /// rgba_hash: 디코딩된 RGBA 원본 바이트의 sha256 (PNG 인코딩 차이와 무관하게 동일 이미지를 판정)
    Image { png: Vec<u8>, thumb_png: Vec<u8>, rgba_hash: String },
}

impl ClipContent {
    fn hash(&self) -> String {
        let mut h = Sha256::new();
        match self {
            Self::Text(t) => {
                h.update(b"text\0");
                h.update(t.as_bytes());
            }
            Self::Files(f) => {
                h.update(b"files\0");
                h.update(f.join("\n").as_bytes());
            }
            Self::Image { rgba_hash, .. } => {
                h.update(b"image\0");
                h.update(rgba_hash.as_bytes());
            }
        }
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipItem {
    pub id: i64,
    pub kind: String,
    pub text: Option<String>,
    pub files: Option<Vec<String>>,
    /// `data:image/png;base64,...`
    pub thumbnail: Option<String>,
    pub pinned: bool,
    pub last_used_at: i64,
}

/// (id, kind, text, image_file, pinned, last_used_at)
type Row = (i64, String, Option<String>, Option<String>, bool, i64);

pub struct ClipStore {
    conn: Connection,
    images_dir: PathBuf,
}

fn io_err(e: std::io::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(e))
}

impl ClipStore {
    pub fn open(db_path: &Path, images_dir: &Path) -> rusqlite::Result<Self> {
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).map_err(io_err)?;
        }
        Self::init(Connection::open(db_path)?, images_dir)
    }

    #[cfg(test)]
    pub fn open_in_memory(images_dir: &Path) -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?, images_dir)
    }

    fn init(conn: Connection, images_dir: &Path) -> rusqlite::Result<Self> {
        fs::create_dir_all(images_dir).map_err(io_err)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn, images_dir: images_dir.to_path_buf() })
    }

    /// 같은 hash 가 있으면 last_used_at 만 갱신해 맨 위로 올린다.
    pub fn record(&mut self, content: ClipContent, now_ms: i64, max_items: u32) -> rusqlite::Result<i64> {
        let hash = content.hash();
        if let Some(id) = self.find_by_hash(&hash)? {
            self.conn.execute("UPDATE clips SET last_used_at = ?1 WHERE id = ?2", params![now_ms, id])?;
            return Ok(id);
        }
        let id = self.insert(content, &hash, now_ms)?;
        self.prune(max_items)?;
        Ok(id)
    }

    fn find_by_hash(&self, hash: &str) -> rusqlite::Result<Option<i64>> {
        self.conn.query_row("SELECT id FROM clips WHERE hash = ?1", [hash], |r| r.get(0)).optional()
    }

    fn insert(&mut self, content: ClipContent, hash: &str, now_ms: i64) -> rusqlite::Result<i64> {
        let (kind, text, image_file) = match content {
            ClipContent::Text(t) => ("text", Some(t), None),
            ClipContent::Files(f) => ("files", Some(f.join("\n")), None),
            ClipContent::Image { png, thumb_png, .. } => {
                let uuid = uuid::Uuid::new_v4().to_string();
                fs::write(self.image_path(&uuid), png).map_err(io_err)?;
                fs::write(self.thumb_path(&uuid), thumb_png).map_err(io_err)?;
                ("image", None, Some(uuid))
            }
        };
        let result = self.conn.execute(
            "INSERT INTO clips (kind, text, image_file, hash, last_used_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![kind, text, image_file, hash, now_ms],
        );
        match result {
            Ok(_) => Ok(self.conn.last_insert_rowid()),
            Err(e) => {
                // 행 삽입이 실패하면 방금 쓴 파일이 고아가 되므로 지운다
                if let Some(u) = &image_file {
                    self.remove_image_files(u);
                }
                Err(e)
            }
        }
    }

    /// 고정되지 않은 항목을 최신순 max_items 개만 남긴다.
    pub fn prune(&mut self, max_items: u32) -> rusqlite::Result<()> {
        let victims: Vec<(i64, Option<String>)> = self
            .conn
            .prepare("SELECT id, image_file FROM clips WHERE pinned = 0 ORDER BY last_used_at DESC LIMIT -1 OFFSET ?1")?
            .query_map([max_items], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        for (id, image) in victims {
            self.delete_row(id, image.as_deref())?;
        }
        Ok(())
    }

    pub fn list(&self, query: Option<&str>, limit: u32) -> rusqlite::Result<Vec<ClipItem>> {
        let pattern = query.filter(|q| !q.is_empty()).map(|q| {
            let escaped = q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
            format!("%{escaped}%")
        });
        // 검색은 텍스트·파일 항목만 대상 (이미지는 text 가 NULL 이라 LIKE 에서 빠진다)
        let sql = format!(
            "SELECT id, kind, text, image_file, pinned, last_used_at FROM clips {} ORDER BY last_used_at DESC, id DESC LIMIT ?1",
            if pattern.is_some() { "WHERE text LIKE ?2 ESCAPE '\\'" } else { "" }
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let map = |r: &rusqlite::Row| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, bool>(4)?,
                r.get::<_, i64>(5)?,
            ))
        };
        let rows: Vec<_> = match &pattern {
            Some(p) => stmt.query_map(params![limit, p], map)?.collect::<Result<_, _>>()?,
            None => stmt.query_map(params![limit], map)?.collect::<Result<_, _>>()?,
        };
        Ok(rows.into_iter().map(|row| self.build_item(row)).collect())
    }

    /// 붙여넣기용 단건 조회. 이미지면 원본 PNG 경로를 함께 돌려준다.
    pub fn get(&self, id: i64) -> rusqlite::Result<Option<(ClipItem, Option<PathBuf>)>> {
        let row: Option<Row> = self
            .conn
            .query_row(
                "SELECT id, kind, text, image_file, pinned, last_used_at FROM clips WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .optional()?;
        Ok(row.map(|row| {
            let path = row.3.as_deref().map(|u| self.image_path(u));
            (self.build_item(row), path)
        }))
    }

    fn build_item(&self, (id, kind, text, image_file, pinned, last_used_at): Row) -> ClipItem {
        let files = (kind == "files").then(|| text.as_deref().unwrap_or("").split('\n').map(String::from).collect());
        let thumbnail = image_file
            .and_then(|u| fs::read(self.thumb_path(&u)).ok())
            .map(|b| format!("data:image/png;base64,{}", STANDARD.encode(b)));
        ClipItem { id, kind, text, files, thumbnail, pinned, last_used_at }
    }

    pub fn set_pinned(&mut self, id: i64, pinned: bool) -> rusqlite::Result<()> {
        self.conn.execute("UPDATE clips SET pinned = ?1 WHERE id = ?2", params![pinned, id])?;
        Ok(())
    }

    pub fn delete(&mut self, id: i64) -> rusqlite::Result<()> {
        let image: Option<Option<String>> =
            self.conn.query_row("SELECT image_file FROM clips WHERE id = ?1", [id], |r| r.get(0)).optional()?;
        if let Some(image) = image {
            self.delete_row(id, image.as_deref())?;
        }
        Ok(())
    }

    pub fn clear_unpinned(&mut self) -> rusqlite::Result<()> {
        self.prune(0)
    }

    fn delete_row(&mut self, id: i64, image_file: Option<&str>) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM clips WHERE id = ?1", [id])?;
        if let Some(u) = image_file {
            self.remove_image_files(u);
        }
        Ok(())
    }

    fn remove_image_files(&self, uuid: &str) {
        let _ = fs::remove_file(self.image_path(uuid));
        let _ = fs::remove_file(self.thumb_path(uuid));
    }

    fn image_path(&self, uuid: &str) -> PathBuf {
        self.images_dir.join(format!("{uuid}.png"))
    }

    fn thumb_path(&self, uuid: &str) -> PathBuf {
        self.images_dir.join(format!("{uuid}.thumb.png"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (ClipStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        (ClipStore::open_in_memory(dir.path()).unwrap(), dir)
    }

    #[test]
    fn duplicate_moves_to_top_without_new_row() {
        let (mut s, _d) = store();
        let a = s.record(ClipContent::Text("a".into()), 1, 500).unwrap();
        s.record(ClipContent::Text("b".into()), 2, 500).unwrap();
        assert_eq!(s.record(ClipContent::Text("a".into()), 3, 500).unwrap(), a);
        let ids: Vec<i64> = s.list(None, 200).unwrap().iter().map(|c| c.id).collect();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], a);
    }

    #[test]
    fn prune_keeps_pinned_and_newest_unpinned() {
        let (mut s, _d) = store();
        let old = s.record(ClipContent::Text("old".into()), 1, 500).unwrap();
        s.set_pinned(old, true).unwrap();
        for i in 2..=5 {
            s.record(ClipContent::Text(format!("t{i}")), i, 2).unwrap();
        }
        let texts: Vec<String> = s.list(None, 200).unwrap().into_iter().map(|c| c.text.unwrap()).collect();
        assert_eq!(texts, vec!["t5", "t4", "old"]);
    }

    #[test]
    fn search_escapes_like_wildcards() {
        let (mut s, _d) = store();
        s.record(ClipContent::Text("100%".into()), 1, 500).unwrap();
        s.record(ClipContent::Text("1000".into()), 2, 500).unwrap();
        assert_eq!(s.list(Some("0%"), 200).unwrap().len(), 1);
    }

    #[test]
    fn clear_keeps_pinned_and_removes_image_files() {
        let (mut s, d) = store();
        let img = s.record(ClipContent::Image { png: vec![1], thumb_png: vec![2], rgba_hash: "h".into() }, 1, 500).unwrap();
        let pinned = s.record(ClipContent::Text("keep".into()), 2, 500).unwrap();
        s.set_pinned(pinned, true).unwrap();
        s.clear_unpinned().unwrap();
        assert_eq!(s.list(None, 200).unwrap().len(), 1);
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 0, "image {img} files removed");
    }

    #[test]
    fn files_roundtrip_and_image_thumbnail_data_uri() {
        let (mut s, _d) = store();
        s.record(ClipContent::Files(vec!["C:\\a.txt".into(), "C:\\b.txt".into()]), 1, 500).unwrap();
        s.record(ClipContent::Image { png: vec![1], thumb_png: vec![2], rgba_hash: "h".into() }, 2, 500).unwrap();
        let items = s.list(None, 200).unwrap();
        assert!(items[0].thumbnail.as_deref().unwrap().starts_with("data:image/png;base64,"));
        assert_eq!(items[1].files.as_ref().unwrap().len(), 2);
        assert_eq!(s.list(Some("a.txt"), 200).unwrap().len(), 1);
    }

    #[test]
    fn get_returns_item_and_image_path() {
        let (mut s, d) = store();
        let t = s.record(ClipContent::Text("hi".into()), 1, 500).unwrap();
        let i = s.record(ClipContent::Image { png: vec![1], thumb_png: vec![2], rgba_hash: "h".into() }, 2, 500).unwrap();
        let (item, path) = s.get(t).unwrap().unwrap();
        assert_eq!((item.text.as_deref(), path), (Some("hi"), None));
        let (_, path) = s.get(i).unwrap().unwrap();
        assert!(path.unwrap().starts_with(d.path()));
        assert!(s.get(999).unwrap().is_none());
    }

    #[test]
    fn delete_removes_row_and_files() {
        let (mut s, d) = store();
        let id = s.record(ClipContent::Image { png: vec![1], thumb_png: vec![2], rgba_hash: "h".into() }, 1, 500).unwrap();
        s.delete(id).unwrap();
        assert!(s.list(None, 200).unwrap().is_empty());
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 0);
    }
}
