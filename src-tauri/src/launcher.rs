use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::platform::current::process;
use crate::settings::model::{LaunchItem, LaunchTarget, OsPath};

#[derive(Debug, Clone, PartialEq)]
pub struct WinInfo {
    pub hwnd: isize,
    pub exe: String,
}

#[derive(Debug, PartialEq)]
pub enum Decision {
    Launch,
    Focus(isize),
    Minimize(isize),
}

/// windows 는 Z-order(앞→뒤) 순이어야 한다. exe 경로는 대소문자를 무시하고 비교한다.
pub fn decide(target_exe: &str, windows: &[WinInfo], foreground: Option<isize>) -> Decision {
    let target = target_exe.to_lowercase();
    let mine: Vec<&WinInfo> = windows.iter().filter(|w| w.exe.to_lowercase() == target).collect();
    match mine.first() {
        None => Decision::Launch,
        Some(_) if foreground.is_some_and(|f| mine.iter().any(|w| w.hwnd == f)) => {
            Decision::Minimize(foreground.unwrap())
        }
        Some(w) => Decision::Focus(w.hwnd),
    }
}

pub fn run(app: &AppHandle, item: &LaunchItem) -> anyhow::Result<()> {
    match &item.target {
        LaunchTarget::App { path } => run_app(app, path),
        LaunchTarget::Url { url, browser } => open_url(app, url, browser),
    }
}

fn run_app(app: &AppHandle, path: &OsPath) -> anyhow::Result<()> {
    let Some(path) = path.current() else { bail!("현재 OS 에 지정된 앱 경로가 없다") };
    if !Path::new(path).exists() {
        bail!("앱 경로가 없다: {path}");
    }
    // 대상 exe 를 모르면(스토어 앱 등) 토글 없이 항상 실행한다
    let Some(exe) = resolve_exe(path) else { return launch(app, path) };

    let windows: Vec<WinInfo> = process::top_windows()?
        .into_iter()
        .map(|(hwnd, exe)| WinInfo { hwnd, exe })
        .collect();
    let foreground = crate::platform::current::window_ops::foreground().ok();
    match decide(&exe, &windows, foreground) {
        Decision::Launch if should_launch(&mut recent_launches().lock().unwrap_or_else(|e| e.into_inner()), &exe, Instant::now()) => {
            launch(app, path)
        }
        Decision::Launch => Ok(()),
        Decision::Focus(h) => process::focus(h),
        Decision::Minimize(h) => process::minimize(h),
    }
}

/// 연타·창이 뜨기 전 재입력으로 같은 앱이 여러 번 실행되지 않도록 대상별 직전 실행 시각을 기억한다.
const LAUNCH_DEBOUNCE: Duration = Duration::from_secs(3);

fn recent_launches() -> &'static Mutex<HashMap<String, Instant>> {
    static M: std::sync::OnceLock<Mutex<HashMap<String, Instant>>> = std::sync::OnceLock::new();
    M.get_or_init(Default::default)
}

/// 실행해도 되면 true 이고 시각을 기록한다. 경로는 대소문자를 무시한다.
fn should_launch(recent: &mut HashMap<String, Instant>, target: &str, now: Instant) -> bool {
    let key = target.to_lowercase();
    if recent.get(&key).is_some_and(|t| now.saturating_duration_since(*t) < LAUNCH_DEBOUNCE) {
        return false;
    }
    recent.insert(key, now);
    true
}

fn launch(app: &AppHandle, path: &str) -> anyhow::Result<()> {
    app.opener().open_path(path, None::<&str>).with_context(|| format!("실행 실패: {path}"))
}

/// .lnk 면 대상 exe, 아니면 경로 자체. 대상을 못 구하면 None.
fn resolve_exe(path: &str) -> Option<String> {
    if !path.to_lowercase().ends_with(".lnk") {
        return Some(path.to_string());
    }
    // lnk 0.6 의 link_target 은 일부 구조에서 panic 하므로 막는다
    let target = std::panic::catch_unwind(|| {
        lnk::ShellLink::open(path, encoding_rs::EUC_KR).ok()?.link_target()
    })
    .ok()??;
    Some(target).filter(|t| !t.is_empty())
}

pub fn open_url(app: &AppHandle, url: &str, browser: &OsPath) -> anyhow::Result<()> {
    match browser.current() {
        // macOS 의 브라우저 경로는 .app 번들(디렉터리)이라 직접 실행할 수 없다
        #[cfg(target_os = "macos")]
        Some(app_path) => {
            std::process::Command::new("open")
                .args(["-a", app_path, url])
                .spawn()
                .with_context(|| format!("브라우저 실행 실패: {app_path}"))?;
        }
        #[cfg(not(target_os = "macos"))]
        Some(exe) => {
            std::process::Command::new(exe).arg(url).spawn().with_context(|| format!("브라우저 실행 실패: {exe}"))?;
        }
        None => app.opener().open_url(url, None::<&str>)?,
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InstalledApp {
    pub name: String,
    pub path: String,
}

/// Windows 는 시작 메뉴 바로가기(.lnk), macOS 는 응용 프로그램 폴더의 앱 번들(.app).
#[cfg(not(target_os = "macos"))]
pub fn scan_installed() -> Vec<InstalledApp> {
    let rel = Path::new("Microsoft").join("Windows").join("Start Menu").join("Programs");
    let root = |var: &str| std::env::var_os(var).map(|v| PathBuf::from(v).join(&rel)).unwrap_or_default();
    scan_dirs(&[root("APPDATA"), root("ProgramData")], "lnk")
}

#[cfg(target_os = "macos")]
pub fn scan_installed() -> Vec<InstalledApp> {
    let mut roots = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join("Applications"));
    }
    roots.extend(["/Applications", "/System/Applications"].map(PathBuf::from));
    scan_dirs(&roots, "app")
}

/// 앞쪽 폴더를 먼저 훑어 같은 이름은 앞쪽(사용자) 항목이 남는다. 결과는 이름순.
pub fn scan_dirs(roots: &[PathBuf], ext: &str) -> Vec<InstalledApp> {
    let mut seen = HashSet::new();
    let mut apps = Vec::new();
    for root in roots {
        let mut found = Vec::new();
        collect(root, ext, &mut found);
        for path in found {
            let Some(name) = path.file_stem().and_then(|s| s.to_str()) else { continue };
            let lower = name.to_lowercase();
            if lower.contains("uninstall") || lower.contains("제거") || !seen.insert(lower) {
                continue;
            }
            apps.push(InstalledApp { name: name.to_string(), path: path.to_string_lossy().into_owned() });
        }
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

/// 확장자를 먼저 본다: .app 은 디렉터리라 안으로 내려가면 번들 내부의 도우미 앱까지 잡힌다.
fn collect(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)) {
            out.push(p);
        } else if p.is_dir() {
            collect(&p, ext, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn launch_debounce_blocks_within_3s_per_target() {
        let mut m = HashMap::new();
        let t0 = Instant::now();
        assert!(should_launch(&mut m, "C:/A.exe", t0));
        assert!(!should_launch(&mut m, "c:/a.exe", t0 + Duration::from_secs(2)));
        assert!(should_launch(&mut m, "C:/B.exe", t0 + Duration::from_secs(2)));
        assert!(should_launch(&mut m, "C:/A.exe", t0 + Duration::from_secs(3)));
    }

    fn w(h: isize, exe: &str) -> WinInfo {
        WinInfo { hwnd: h, exe: exe.into() }
    }

    #[test]
    fn launch_when_not_running() {
        assert_eq!(decide(r"C:\a\slack.exe", &[w(1, r"C:\b\code.exe")], Some(1)), Decision::Launch);
    }

    #[test]
    fn focus_topmost_window_when_not_foreground() {
        let ws = [w(5, r"C:\A\Slack.exe"), w(6, r"c:\a\slack.exe")];
        assert_eq!(decide(r"C:\a\slack.exe", &ws, Some(99)), Decision::Focus(5));
    }

    #[test]
    fn minimize_when_any_own_window_is_foreground() {
        let ws = [w(5, r"C:\a\slack.exe"), w(6, r"C:\a\slack.exe")];
        assert_eq!(decide(r"C:\a\slack.exe", &ws, Some(6)), Decision::Minimize(6));
    }

    fn touch(root: &Path, rel: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, b"").unwrap();
    }

    #[test]
    fn scan_sorts_by_name_and_recurses() {
        let (u, c) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        touch(u.path(), "Zoom.lnk");
        touch(c.path(), "Dev/Code.lnk");
        touch(c.path(), "alpha.LNK");
        touch(c.path(), "readme.txt");
        let names: Vec<_> = scan_dirs(&[u.path().into(), c.path().into()], "lnk").into_iter().map(|a| a.name).collect();
        assert_eq!(names, ["alpha", "Code", "Zoom"]);
    }

    #[test]
    fn scan_prefers_user_folder_on_duplicate_name() {
        let (u, c) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        touch(u.path(), "Slack.lnk");
        touch(c.path(), "Slack.lnk");
        let apps = scan_dirs(&[u.path().into(), c.path().into()], "lnk");
        assert_eq!(apps.len(), 1);
        assert!(Path::new(&apps[0].path).starts_with(u.path()));
    }

    #[test]
    fn scan_excludes_uninstall_entries() {
        let (u, c) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        touch(u.path(), "Uninstall Foo.lnk");
        touch(c.path(), "Foo 제거.lnk");
        touch(c.path(), "Foo.lnk");
        let names: Vec<_> = scan_dirs(&[u.path().into(), c.path().into()], "lnk").into_iter().map(|a| a.name).collect();
        assert_eq!(names, ["Foo"]);
    }

    #[test]
    fn scan_tolerates_missing_roots() {
        assert!(scan_dirs(&[r"Z:\nope".into(), r"Z:\nope2".into()], "lnk").is_empty());
    }

    #[test]
    fn scan_app_bundles_without_descending_into_them() {
        let root = tempfile::tempdir().unwrap();
        touch(root.path(), "Slack.app/Contents/Frameworks/Slack Helper.app/x");
        touch(root.path(), "Utilities/Terminal.app/Contents/Info.plist");
        let names: Vec<_> = scan_dirs(&[root.path().into()], "app").into_iter().map(|a| a.name).collect();
        assert_eq!(names, ["Slack", "Terminal"]);
    }
}
