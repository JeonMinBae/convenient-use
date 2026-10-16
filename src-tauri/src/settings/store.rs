use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::model::Settings;

pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// 파일이 없으면 기본값. 파싱 실패 시 원본을 `.broken` 으로 보존하고 기본값을 쓴다.
    pub fn load(&self) -> Settings {
        let text = match fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    log::warn!("설정 파일을 읽지 못했습니다: {e}");
                }
                return Settings::default();
            }
        };
        match serde_json::from_str(&text) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("설정 파일이 손상되어 기본값을 사용합니다: {e}");
                let mut broken = self.path.clone().into_os_string();
                broken.push(".broken");
                let _ = fs::rename(&self.path, PathBuf::from(broken));
                Settings::default()
            }
        }
    }

    pub fn save(&self, settings: &Settings) -> Result<()> {
        write_atomic(&self.path, settings)
    }

    pub fn export_to(&self, settings: &Settings, dest: &Path) -> Result<()> {
        write_atomic(dest, settings)
    }

    /// 파싱에 성공한 경우에만 현재 설정 파일을 교체한다.
    pub fn import_from(&self, src: &Path) -> Result<Settings> {
        let text = fs::read_to_string(src).context("가져올 파일을 읽을 수 없습니다")?;
        let settings: Settings = serde_json::from_str(&text).context("설정 파일 형식이 올바르지 않습니다")?;
        self.save(&settings)?;
        Ok(settings)
    }
}

// 임시 파일에 쓴 뒤 rename 해서 쓰기 도중 중단되어도 기존 파일이 깨지지 않게 한다
fn write_atomic(dest: &Path, settings: &Settings) -> Result<()> {
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut tmp = dest.to_path_buf().into_os_string();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    fs::rename(&tmp, dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(dir: &tempfile::TempDir) -> SettingsStore {
        SettingsStore::new(dir.path().join("cfg").join("settings.json"))
    }

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(store(&dir).load(), Settings::default());
    }

    #[test]
    fn save_then_load() {
        let dir = tempfile::tempdir().unwrap();
        let st = store(&dir);
        let mut s = Settings::default();
        s.clipboard.max_items = 42;
        st.save(&s).unwrap();
        assert_eq!(st.load(), s);
    }

    #[test]
    fn broken_file_is_moved_aside() {
        let dir = tempfile::tempdir().unwrap();
        let st = store(&dir);
        fs::create_dir_all(st.path.parent().unwrap()).unwrap();
        fs::write(&st.path, "{ not json").unwrap();
        assert_eq!(st.load(), Settings::default());
        assert!(!st.path.exists());
        let broken = st.path.with_file_name("settings.json.broken");
        assert_eq!(fs::read_to_string(broken).unwrap(), "{ not json");
    }

    #[test]
    fn export_import_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let st = store(&dir);
        let mut s = Settings::default();
        s.general.autostart = false;
        let out = dir.path().join("out.json");
        st.export_to(&s, &out).unwrap();
        let got = st.import_from(&out).unwrap();
        assert_eq!(got, s);
        assert_eq!(st.load(), s);
    }

    #[test]
    fn invalid_import_keeps_current() {
        let dir = tempfile::tempdir().unwrap();
        let st = store(&dir);
        let s = Settings::default();
        st.save(&s).unwrap();
        let bad = dir.path().join("bad.json");
        fs::write(&bad, "nope").unwrap();
        assert!(st.import_from(&bad).is_err());
        assert_eq!(st.load(), s);
    }
}
