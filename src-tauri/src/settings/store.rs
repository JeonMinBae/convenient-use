use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use super::model::Settings;

const READ_ATTEMPTS: u32 = 5;
const READ_RETRY_DELAY: Duration = Duration::from_millis(200);

pub struct SettingsStore {
    path: PathBuf,
    /// 파일이 있는데 읽지 못해 기본값으로 시작했다면 그 사유. 이때 저장하면 원본을 기본값으로 덮어쓰므로 막는다
    save_blocked: Option<String>,
}

impl SettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path, save_blocked: None }
    }

    /// 파일이 없으면 기본값. 파싱 실패 시 원본을 `.broken-<ms>` 로 보존하고 기본값을 쓴다.
    /// 읽기 자체가 실패하면(부팅 직후 백신 잠금 등) 잠시 재시도하고, 끝내 실패하면 저장을 막은 채 기본값을 쓴다.
    pub fn load(&mut self) -> Settings {
        self.save_blocked = None;
        let text = match read_with_retry(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == ErrorKind::NotFound => return Settings::default(),
            Err(e) => {
                log::warn!("설정 파일을 읽지 못했습니다: {e}");
                self.save_blocked = Some(format!(
                    "설정 파일을 읽지 못해 기본값으로 실행 중입니다({e}). 원본을 지키려고 저장하지 않습니다. 앱을 다시 시작하세요"
                ));
                return Settings::default();
            }
        };
        match serde_json::from_str(&text) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("설정 파일이 손상되어 기본값을 사용합니다: {e}");
                // 이전 백업을 덮어쓰지 않도록 시각을 붙인다
                let ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
                let mut broken = self.path.clone().into_os_string();
                broken.push(format!(".broken-{ms}"));
                let _ = fs::rename(&self.path, PathBuf::from(broken));
                Settings::default()
            }
        }
    }

    /// 시작 시 읽기 실패로 저장이 막혀 있으면 사용자에게 알릴 사유
    pub fn save_blocked(&self) -> Option<&str> {
        self.save_blocked.as_deref()
    }

    pub fn save(&self, settings: &Settings) -> Result<()> {
        if let Some(reason) = &self.save_blocked {
            bail!("{reason}");
        }
        write_atomic(&self.path, settings)
    }

    pub fn export_to(&self, settings: &Settings, dest: &Path) -> Result<()> {
        write_atomic(dest, settings)
    }

    /// 가져올 파일을 파싱만 한다. 검증·저장은 호출자가 한다 (commands::import_validated).
    pub fn read_from(&self, src: &Path) -> Result<Settings> {
        let text = fs::read_to_string(src).context("가져올 파일을 읽을 수 없습니다")?;
        serde_json::from_str(&text).context("설정 파일 형식이 올바르지 않습니다")
    }
}

fn read_with_retry(path: &Path) -> std::io::Result<String> {
    let mut attempt = 1;
    loop {
        match fs::read_to_string(path) {
            Err(e) if e.kind() != ErrorKind::NotFound && attempt < READ_ATTEMPTS => {
                attempt += 1;
                std::thread::sleep(READ_RETRY_DELAY);
            }
            result => return result,
        }
    }
}

// 임시 파일에 쓴 뒤 rename 해서 쓰기 도중 중단되어도 기존 파일이 깨지지 않게 한다.
// rename 전에 sync_all 해야 한다: 안 하면 직후 전원이 끊겼을 때 rename 만 반영되어 빈 파일이 남을 수 있다
fn write_atomic(dest: &Path, settings: &Settings) -> Result<()> {
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut tmp = dest.to_path_buf().into_os_string();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let mut file = fs::File::create(&tmp)?;
    file.write_all(&serde_json::to_vec_pretty(settings)?)?;
    file.sync_all()?;
    drop(file);
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
        let mut st = store(&dir);
        let mut s = Settings::default();
        s.clipboard.max_items = 42;
        st.save(&s).unwrap();
        assert_eq!(st.load(), s);
    }

    fn broken_backups(st: &SettingsStore) -> Vec<String> {
        fs::read_dir(st.path.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.starts_with("settings.json.broken-"))
            .collect()
    }

    #[test]
    fn broken_file_is_moved_aside_without_overwriting_older_backup() {
        let dir = tempfile::tempdir().unwrap();
        let mut st = store(&dir);
        fs::create_dir_all(st.path.parent().unwrap()).unwrap();
        fs::write(&st.path, "{ not json").unwrap();
        assert_eq!(st.load(), Settings::default());
        assert!(!st.path.exists());
        assert!(st.save_blocked().is_none());

        std::thread::sleep(Duration::from_millis(5));
        fs::write(&st.path, "").unwrap();
        st.load();
        let backups = broken_backups(&st);
        assert_eq!(backups.len(), 2);
        let contents: Vec<_> =
            backups.iter().map(|n| fs::read_to_string(st.path.with_file_name(n)).unwrap()).collect();
        assert!(contents.contains(&"{ not json".to_string()));
    }

    #[test]
    fn unreadable_file_blocks_save_and_keeps_original() {
        let dir = tempfile::tempdir().unwrap();
        let mut st = store(&dir);
        // 디렉터리는 NotFound 가 아닌 읽기 오류를 낸다
        fs::create_dir_all(&st.path).unwrap();
        assert_eq!(st.load(), Settings::default());
        assert!(st.save_blocked().is_some());
        assert!(st.save(&Settings::default()).is_err());
        assert!(st.path.is_dir());
    }

    #[test]
    fn export_import_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let st = store(&dir);
        let mut s = Settings::default();
        s.general.autostart = false;
        let out = dir.path().join("out.json");
        st.export_to(&s, &out).unwrap();
        assert_eq!(st.read_from(&out).unwrap(), s);
    }

    #[test]
    fn invalid_import_keeps_current() {
        let dir = tempfile::tempdir().unwrap();
        let mut st = store(&dir);
        let s = Settings::default();
        st.save(&s).unwrap();
        let bad = dir.path().join("bad.json");
        fs::write(&bad, "nope").unwrap();
        assert!(st.read_from(&bad).is_err());
        assert_eq!(st.load(), s);
    }
}
