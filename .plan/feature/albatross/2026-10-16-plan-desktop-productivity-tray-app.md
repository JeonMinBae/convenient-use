# 작업 계획: Windows/macOS 생산성 트레이 앱

## 기준

- 요구사항: `.plan/feature/albatross/2026-10-16-requirement-desktop-productivity-tray-app.md`
- 대상: 현재 워크트리(저장소 루트), 브랜치 `feature/albatross`
- 현재 상태: 빈 저장소(Initial commit). 참고할 기존 구현 없음
- 도구: Rust 1.99(MSVC), Node 26 / npm 11, Tauri 2.12.x(안정판, 3.x alpha는 쓰지 않는다), Vite + React 19 + TypeScript
- 구현 범위: Windows. macOS는 `platform` 모듈의 미지원 스텁으로 컴파일 경계만 둔다

## 접근 방식

- **순수 로직과 OS 호출 분리**: 설정, 단축키 문자열, 창 배치 계산, 앱 토글 판단, 클립보드 저장소, 스니펫 매처, 리매핑 엔진은 OS 비의존 Rust 모듈로 두고 `cargo test`로 TDD 한다. Win32 호출은 `src-tauri/src/platform/windows/` 에만 둔다.
- **OS 부작용은 수동 검증**: 창 이동, 포커스, 키 주입, 전역 훅은 자동 테스트 범위 밖이다. 해당 단위의 성공 기준에 수동 확인 항목을 적는다.
- **설정이 단일 진실 원천**: 모든 기능은 `Settings` 를 읽고, 저장 시 `apply_settings` 한 곳에서 단축키 재등록·클립보드 정리·엔진 갱신·자동 실행을 반영한다.
- **YAGNI**: UI 라이브러리 없이 React + CSS. 상태 관리 라이브러리 없음. 요구사항 밖 기능(검색 런처, 드래그 스냅, 민감정보 필터)은 만들지 않는다.
- **단축키 표기 통일**: 설정에는 `Ctrl+Alt+Shift+Super+<Code>` 순서로 정규화한 문자열을 저장한다. `<Code>` 는 W3C `KeyboardEvent.code` 이름(`KeyU`, `ArrowLeft`, `Enter`, `Digit1`)이다. 리매핑 키 이름도 같은 Code 이름을 쓴다.

## 해석 결정 (요구사항 범위 내 세부)

- 클립보드 최대 보관 개수는 **고정되지 않은 항목 수** 기준이다. 고정 항목은 개수에 포함하지 않는다.
- 클립보드 목록 정렬은 마지막 복사/사용 시각 내림차순이다.
- 이미지 크기 제한은 PNG 인코딩 후 파일 크기로 판단한다.
- 스니펫 키워드는 즉시 치환이므로 한 키워드가 다른 키워드의 접두어이면 저장을 막는다(긴 쪽에 도달할 수 없음). 동시에 끝나는 키워드가 여럿이면 가장 긴 것을 쓴다.
- 탭/홀드: 다른 키를 함께 누르면 홀드, 아무 키 없이 200ms 안에 떼면 탭, 200ms 이후에 떼면 아무것도 보내지 않는다.

## 파일 목록

### 생성

| 경로 | 책임 |
|---|---|
| `package.json`, `vite.config.ts`, `tsconfig.json`, `index.html` | 프론트 빌드 |
| `src/main.tsx` | 창 label 에 따라 설정 화면 / 클립보드 팝업 렌더 |
| `src/api.ts` | Tauri command 호출 래퍼와 공용 타입 |
| `src/lib/accelerator.ts`, `src/lib/accelerator.test.ts` | KeyboardEvent → 단축키 문자열 |
| `src/components/HotkeyInput.tsx` | 단축키 녹화 입력 |
| `src/settings/SettingsApp.tsx` | 설정 창 탭 컨테이너, 저장/오류 표시 |
| `src/settings/GeneralTab.tsx` | 자동 실행, 기능 토글, 가져오기/내보내기 |
| `src/settings/LaunchTab.tsx` | 실행 단축키 목록 |
| `src/settings/WindowTab.tsx` | 창 배치 단축키 |
| `src/settings/ClipboardTab.tsx` | 클립보드 설정 |
| `src/settings/SnippetTab.tsx` | 스니펫 목록 |
| `src/settings/RemapTab.tsx` | 리매핑 규칙, 제외 앱 |
| `src/popup/ClipboardPopup.tsx` | 클립보드 검색 팝업 |
| `src/styles.css` | 공용 스타일 |
| `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/icons/*` | Tauri 앱 |
| `src-tauri/src/main.rs`, `src-tauri/src/lib.rs` | 진입점, 플러그인·상태·트레이 조립 |
| `src-tauri/src/state.rs` | `AppState` |
| `src-tauri/src/tray.rs` | 트레이 메뉴 |
| `src-tauri/src/commands.rs` | 프론트용 Tauri command |
| `src-tauri/src/settings/{mod,model,store}.rs` | 설정 모델, 기본값, 파일 저장/가져오기/내보내기, 검증 |
| `src-tauri/src/hotkey/{mod,accelerator,registry}.rs` | 단축키 파싱·정규화, 바인딩 수집·충돌 검사, 등록·디스패치 |
| `src-tauri/src/window_layout.rs` | 창 배치 명령과 사각형 계산 |
| `src-tauri/src/launcher.rs` | 앱/URL 실행, 토글 판단, 설치 앱 목록 |
| `src-tauri/src/clipboard/{mod,store,watcher,popup}.rs` | SQLite 저장소, 변경 감시, 팝업·붙여넣기 |
| `src-tauri/src/keyboard/{mod,keys}.rs` | 전역 키 이벤트 타입, Code↔VK 변환, 핸들러 체인 |
| `src-tauri/src/snippet.rs` | 스니펫 매처와 확장 |
| `src-tauri/src/remap.rs` | 리매핑 엔진 |
| `src-tauri/src/platform/{mod,unsupported}.rs` | OS 분기 |
| `src-tauri/src/platform/windows/{mod,window_ops,process,input,hook}.rs` | Win32 창·프로세스·SendInput·LL 키보드 훅 |
| `.gitignore` | `node_modules`, `dist`, `src-tauri/target` |

### 수정

없음 (빈 저장소)

### 삭제

없음

---

## 작업 단위

### [ ] Task 1. Tauri + React 프로젝트 골격과 트레이 상주

```text
lib.rs#run()                                [NEW]
  tauri_plugin_single_instance::init()      // framework boundary
  tray::create()                            [NEW]
    TrayIconBuilder                         // framework boundary
  tray::on_menu_event()                     [NEW]
    tray::show_settings()                   [NEW]   // "설정 열기"
    AppHandle#exit()                        // "종료"
```

- [ ] 임시 디렉터리에서 `npm create tauri-app@4.7.5 -- convenient-use --template react-ts --manager npm --yes` 로 생성 후 저장소 루트로 복사(`.git` 은 덮어쓰지 않는다)
- [ ] `src-tauri/Cargo.toml`: `tauri = { version = "2.12", features = ["tray-icon", "image-png"] }`, `tauri-plugin-single-instance = "2"`, `serde`, `serde_json`, `anyhow`, `log`
- [ ] `tauri.conf.json`: identifier `com.convenientuse.app`, productName `Convenient Use`, 창 2개 정의(`settings` 760x560 숨김 시작, `clipboard-popup` 640x460 `decorations:false` `alwaysOnTop:true` `skipTaskbar:true` 숨김 시작)
- [ ] `src-tauri/src/tray.rs`: 메뉴 "설정 열기", "종료". 아이콘 왼쪽 클릭도 설정 열기
- [ ] 설정 창 닫기 요청은 종료하지 않고 숨김(`CloseRequested` → `prevent_close` + `hide`)
- [ ] `src/main.tsx`: `getCurrentWindow().label` 이 `clipboard-popup` 이면 `ClipboardPopup`, 아니면 `SettingsApp` (Task 1 에서는 두 컴포넌트를 제목만 있는 최소 버전으로 생성)
- [ ] `src-tauri/src/platform/mod.rs`, `unsupported.rs`, `windows/mod.rs` 빈 모듈 경계 생성
- [ ] `.gitignore`
- [ ] 성공 기준
  - [ ] `npm run build` 성공
  - [ ] `cargo check` (src-tauri) 성공
  - [ ] 수동: `npm run tauri dev` → 트레이 아이콘, 설정 열기/닫기(숨김), 종료 동작
  - [ ] 수동: 두 번째 실행 시 새 인스턴스 없이 기존 설정 창 표시

`src-tauri/src/lib.rs`

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_settings(app);
        }))
        .setup(|app| {
            tray::create(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // 상주 앱이므로 창 닫기는 숨김으로 처리한다
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

`src-tauri/src/platform/mod.rs`

```rust
#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use self::windows as current;

#[cfg(not(windows))]
mod unsupported;
#[cfg(not(windows))]
pub use self::unsupported as current;
```

---

### [ ] Task 2. 설정 모델, 기본값, 저장·가져오기·내보내기

```text
lib.rs#run()                                  [MOD]
  settings::store::SettingsStore#load()       [NEW]
    settings::model::Settings#default()       [NEW]   // 파일 없음
commands::get_settings()                      [NEW]
commands::export_settings()                   [NEW]
  SettingsStore#export_to()                   [NEW]
commands::import_settings()                   [NEW]
  SettingsStore#import_from()                 [NEW]
    SettingsStore#save()                      [NEW]
      std::fs::rename                         // fs boundary
```

- [ ] 테스트 먼저: `src-tauri/src/settings/model.rs` 하단 `#[cfg(test)]`
- [ ] `settings/model.rs`: `Settings` 와 하위 타입, `Default` 구현(Rectangle 프리셋, 클립보드 500/10MB, 자동 실행 true, 모든 기능 켬)
- [ ] OS별 경로는 `OsPath { windows: Option<String>, macos: Option<String> }` 로 저장하고 `OsPath::current()` 로 현재 OS 값만 읽는다
- [ ] 모든 구조체에 `#[serde(default)]` 를 달아 필드가 빠진 파일·이전 버전 파일을 읽을 수 있게 한다
- [ ] `settings/store.rs`: 경로 `app.path().app_config_dir()/settings.json`. 저장은 임시 파일 쓰기 후 rename. 파싱 실패 시 기존 파일을 `settings.json.broken` 으로 옮기고 기본값 사용
- [ ] `AppState { settings: RwLock<Settings>, store: SettingsStore }` (`state.rs`), `commands.rs` 에 `get_settings`, `export_settings(path)`, `import_settings(path)`
- [ ] 성공 기준
  - [ ] 테스트 통과
  - [ ] 기본값 직렬화 → 역직렬화 동일
  - [ ] 다른 OS 경로만 있는 항목은 현재 OS 에서 경로 없음

`src-tauri/src/settings/model.rs`

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub general: GeneralSettings,
    pub launch_items: Vec<LaunchItem>,
    pub window_hotkeys: Vec<WindowHotkey>,
    pub clipboard: ClipboardSettings,
    pub snippets: Vec<Snippet>,
    pub remap: RemapSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GeneralSettings {
    pub autostart: bool,             // 기본 true
    pub features: FeatureToggles,    // launch/window/clipboard/snippet/remap 모두 기본 true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchItem {
    pub id: String,                  // uuid
    pub name: String,
    pub hotkey: String,
    pub target: LaunchTarget,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum LaunchTarget {
    App { path: OsPath },
    Url { url: String, browser: OsPath }, // browser 미지정이면 기본 브라우저
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowHotkey { pub command: LayoutCommand, pub hotkey: String }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ClipboardSettings {
    pub hotkey: String,              // Windows "Ctrl+Shift+KeyV", macOS "Shift+Super+KeyV"
    pub max_items: u32,              // 500
    pub max_image_mb: u32,           // 10
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet { pub id: String, pub keyword: String, pub text: String }

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RemapSettings {
    pub rules: Vec<RemapRule>,
    pub excluded_apps: Vec<String>,  // 실행 파일 이름, 소문자 비교 (예: "valorant.exe")
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OsPath { pub windows: Option<String>, pub macos: Option<String> }

impl OsPath {
    pub fn current(&self) -> Option<&str> {
        let v = if cfg!(windows) { &self.windows } else { &self.macos };
        v.as_deref().filter(|s| !s.is_empty())
    }
}
```

`LayoutCommand` 는 Task 3, `RemapRule` 은 Task 11 에서 정의한다. Task 2 시점에는 `window_hotkeys` 와 `remap.rules` 를 위해 두 타입을 아래 형태로 이 단위에서 먼저 만든다(이후 단위는 사용만 한다).

```rust
// src-tauri/src/window_layout.rs (Task 2 에서 enum 만 생성)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LayoutCommand {
    LeftHalf, RightHalf, TopHalf, BottomHalf,
    TopLeft, TopRight, BottomLeft, BottomRight,
    FirstThird, CenterThird, LastThird, FirstTwoThirds, LastTwoThirds,
    Maximize, Center, NextDisplay,
}

// src-tauri/src/remap.rs (Task 2 에서 타입만 생성)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyCombo { pub mods: Vec<String>, pub key: Option<String> } // mods: "Ctrl"|"Alt"|"Shift"|"Super"

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RemapRule {
    Simple { from: String, to: KeyCombo },
    TapHold { key: String, tap: KeyCombo, hold: KeyCombo },
    Layer { trigger: String, tap: Option<KeyCombo>, mappings: Vec<LayerMapping> },
    Macro { trigger: String, steps: Vec<KeyCombo> },   // trigger 는 단축키 문자열
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerMapping { pub from: String, pub to: KeyCombo }
```

기본 창 단축키 (Windows; macOS 는 `Super` 자리에 `Super`, `Alt` 를 Option 으로 같은 문자열 사용)

```rust
fn default_window_hotkeys() -> Vec<WindowHotkey> {
    use LayoutCommand::*;
    [
        (LeftHalf, "Ctrl+Alt+ArrowLeft"), (RightHalf, "Ctrl+Alt+ArrowRight"),
        (TopHalf, "Ctrl+Alt+ArrowUp"), (BottomHalf, "Ctrl+Alt+ArrowDown"),
        (TopLeft, "Ctrl+Alt+KeyU"), (TopRight, "Ctrl+Alt+KeyI"),
        (BottomLeft, "Ctrl+Alt+KeyJ"), (BottomRight, "Ctrl+Alt+KeyK"),
        (FirstThird, "Ctrl+Alt+KeyD"), (CenterThird, "Ctrl+Alt+KeyF"), (LastThird, "Ctrl+Alt+KeyG"),
        (FirstTwoThirds, "Ctrl+Alt+KeyE"), (LastTwoThirds, "Ctrl+Alt+KeyT"),
        (Maximize, "Ctrl+Alt+Enter"), (Center, "Ctrl+Alt+KeyC"),
        (NextDisplay, "Ctrl+Alt+Super+ArrowRight"),
    ]
    .into_iter()
    .map(|(command, hotkey)| WindowHotkey { command, hotkey: hotkey.into() })
    .collect()
}
```

테스트

```rust
#[test]
fn defaults_roundtrip() {
    let s = Settings::default();
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), s);
    assert!(s.general.autostart);
    assert_eq!(s.clipboard.max_items, 500);
    assert_eq!(s.clipboard.max_image_mb, 10);
}

#[test]
fn missing_fields_use_defaults() {
    let s: Settings = serde_json::from_str(r#"{"clipboard":{"maxItems":50}}"#).unwrap();
    assert_eq!(s.clipboard.max_items, 50);
    assert_eq!(s.clipboard.max_image_mb, 10);
    assert_eq!(s.window_hotkeys.len(), 16);
}

#[test]
fn other_os_path_is_not_current() {
    let p = OsPath { windows: None, macos: Some("/Applications/Slack.app".into()) };
    if cfg!(windows) { assert_eq!(p.current(), None) }
}
```

---

### [ ] Task 3. 창 배치 계산과 Win32 적용

```text
window_layout::apply()                            [NEW]
  platform::current::window_ops::foreground()     [NEW]
  platform::current::window_ops::monitors()       [NEW]
    EnumDisplayMonitors / GetMonitorInfoW         // Win32 boundary
  window_layout::compute()                        [NEW]
  platform::current::window_ops::set_frame()      [NEW]
    DwmGetWindowAttribute / SetWindowPos          // Win32 boundary
```

- [ ] 테스트 먼저: `window_layout.rs` 하단 `#[cfg(test)]`
- [ ] `window_layout.rs`: `Rect { x, y, w, h }`, `compute(cmd, work: Rect, current: Rect) -> Rect`, `next_display(current, from_work, to_work) -> Rect`, `apply(cmd)`
- [ ] `platform/windows/window_ops.rs`: 포그라운드 창의 프레임/모니터 조회, 최대화 상태면 `SW_RESTORE` 후 이동, DWM 확장 프레임 경계로 보이지 않는 테두리 보정
- [ ] `windows` crate 0.62 features: `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_Graphics_Gdi`, `Win32_Graphics_Dwm`
- [ ] 성공 기준
  - [ ] 테스트 통과
  - [ ] 1/3 분할에서 나머지 픽셀이 마지막 칸에 포함
  - [ ] 다음 모니터 이동 시 상대 위치·비율 유지
  - [ ] 수동: 메모장에서 16개 명령을 각각 호출해 배치 확인(Task 5 이후 단축키로 확인)

`src-tauri/src/window_layout.rs`

```rust
pub fn compute(cmd: LayoutCommand, a: Rect, current: Rect) -> Rect {
    use LayoutCommand::*;
    let (hw, hh) = (a.w / 2, a.h / 2);
    let third = a.w / 3;
    match cmd {
        LeftHalf => Rect { w: hw, ..a },
        RightHalf => Rect { x: a.x + hw, w: a.w - hw, ..a },
        TopHalf => Rect { h: hh, ..a },
        BottomHalf => Rect { y: a.y + hh, h: a.h - hh, ..a },
        TopLeft => Rect { w: hw, h: hh, ..a },
        TopRight => Rect { x: a.x + hw, w: a.w - hw, h: hh, ..a },
        BottomLeft => Rect { y: a.y + hh, w: hw, h: a.h - hh, ..a },
        BottomRight => Rect { x: a.x + hw, y: a.y + hh, w: a.w - hw, h: a.h - hh },
        FirstThird => Rect { w: third, ..a },
        CenterThird => Rect { x: a.x + third, w: third, ..a },
        // 정수 나눗셈 나머지는 마지막 칸이 흡수해 작업 영역 끝까지 채운다
        LastThird => Rect { x: a.x + third * 2, w: a.w - third * 2, ..a },
        FirstTwoThirds => Rect { w: third * 2, ..a },
        LastTwoThirds => Rect { x: a.x + third, w: a.w - third, ..a },
        Maximize => a,
        Center => {
            let w = current.w.min(a.w);
            let h = current.h.min(a.h);
            Rect { x: a.x + (a.w - w) / 2, y: a.y + (a.h - h) / 2, w, h }
        }
        NextDisplay => current, // 모니터 이동은 next_display 가 담당
    }
}

/// 원래 작업 영역 안의 상대 위치·크기 비율을 다음 모니터 작업 영역에 그대로 옮긴다.
pub fn next_display(cur: Rect, from: Rect, to: Rect) -> Rect {
    let sx = |v: i32| (v as i64 * to.w as i64 / from.w as i64) as i32;
    let sy = |v: i32| (v as i64 * to.h as i64 / from.h as i64) as i32;
    Rect { x: to.x + sx(cur.x - from.x), y: to.y + sy(cur.y - from.y), w: sx(cur.w), h: sy(cur.h) }
}
```

`apply` 의 모니터 순서: 작업 영역 `x` 오름차순, 같으면 `y` 오름차순. 현재 모니터 다음 인덱스, 마지막이면 첫 번째로 순환. 모니터가 하나면 아무것도 하지 않는다.

테스트

```rust
const WORK: Rect = Rect { x: 0, y: 40, w: 1001, h: 700 };

#[test]
fn thirds_cover_whole_width() {
    let a = compute(LayoutCommand::FirstThird, WORK, WORK);
    let b = compute(LayoutCommand::CenterThird, WORK, WORK);
    let c = compute(LayoutCommand::LastThird, WORK, WORK);
    assert_eq!((a.x, a.w, b.x, b.w, c.x, c.w), (0, 333, 333, 333, 666, 335));
    assert_eq!(c.x + c.w, WORK.x + WORK.w);
}

#[test]
fn right_half_starts_after_left_half() {
    let l = compute(LayoutCommand::LeftHalf, WORK, WORK);
    let r = compute(LayoutCommand::RightHalf, WORK, WORK);
    assert_eq!(l.x + l.w, r.x);
    assert_eq!(r.x + r.w, 1001);
    assert_eq!(r.y, 40);
}

#[test]
fn center_clamps_oversized_window() {
    let cur = Rect { x: -50, y: 0, w: 3000, h: 300 };
    let c = compute(LayoutCommand::Center, WORK, cur);
    assert_eq!(c, Rect { x: 0, y: 40 + 200, w: 1001, h: 300 });
}

#[test]
fn next_display_keeps_ratio() {
    let from = Rect { x: 0, y: 0, w: 1920, h: 1040 };
    let to = Rect { x: 1920, y: 0, w: 2560, h: 1400 };
    let cur = Rect { x: 0, y: 0, w: 960, h: 1040 };
    assert_eq!(next_display(cur, from, to), Rect { x: 1920, y: 0, w: 1280, h: 1400 });
}
```

---

### [ ] Task 4. 앱/URL 실행과 설치 앱 목록

```text
launcher::run()                                   [NEW]
  launcher::run_app()                             [NEW]
    launcher::resolve_exe()                       [NEW]   // .lnk → 대상 exe
      lnk::ShellLink#open()                       // crate boundary
    platform::current::process::top_windows()     [NEW]
      EnumWindows / QueryFullProcessImageNameW    // Win32 boundary
    launcher::decide()                            [NEW]
    platform::current::process::focus()           [NEW]   // Focus
    platform::current::process::minimize()        [NEW]   // Minimize
    tauri_plugin_opener::open_path()              // Launch, 경로 없음이면 Err
  launcher::open_url()                            [NEW]
    tauri_plugin_opener::open_url()               // 브라우저 미지정
    std::process::Command#spawn()                 // 브라우저 지정
commands::list_installed_apps()                   [NEW]
  launcher::scan_start_menu()                     [NEW]
```

- [ ] 테스트 먼저: `launcher.rs` 하단 `#[cfg(test)]` (`decide`, `scan_start_menu` 의 정렬·중복 제거)
- [ ] 의존성: `tauri-plugin-opener = "2"`, `lnk = "0.6"`, `windows` features 추가 `Win32_System_Threading`, `Win32_System_ProcessStatus`
- [ ] `decide(target_exe, windows, foreground) -> Decision` 순수 함수
- [ ] `focus`: 최소화 상태면 `SW_RESTORE`, 이후 `SetForegroundWindow`. 실패하면 `AttachThreadInput` 으로 포그라운드 스레드에 붙여 재시도
- [ ] `.lnk` 대상 exe 를 못 구하면(스토어 앱 등) 토글 없이 항상 실행
- [ ] `scan_start_menu`: `%APPDATA%\Microsoft\Windows\Start Menu\Programs`, `%ProgramData%\Microsoft\Windows\Start Menu\Programs` 하위 `.lnk` 재귀. 이름 = 파일명(확장자 제외). 이름이 같으면 사용자 폴더 우선, 이름순 정렬. 이름에 `uninstall`/`제거` 포함 항목 제외
- [ ] `commands::list_installed_apps() -> Vec<InstalledApp { name, path }>`
- [ ] 성공 기준
  - [ ] 테스트 통과
  - [ ] 실행 안 됨 → Launch / 비포커스 → Focus / 포커스 → Minimize
  - [ ] exe 경로 비교는 대소문자 무시

`src-tauri/src/launcher.rs`

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct WinInfo { pub hwnd: isize, pub exe: String }

#[derive(Debug, PartialEq)]
pub enum Decision { Launch, Focus(isize), Minimize(isize) }

pub fn decide(target_exe: &str, windows: &[WinInfo], foreground: Option<isize>) -> Decision {
    let target = target_exe.to_lowercase();
    let mine: Vec<&WinInfo> = windows.iter().filter(|w| w.exe.to_lowercase() == target).collect();
    match mine.first() {
        None => Decision::Launch,
        Some(_) if foreground.is_some_and(|f| mine.iter().any(|w| w.hwnd == f)) => {
            Decision::Minimize(foreground.unwrap())
        }
        Some(w) => Decision::Focus(w.hwnd), // EnumWindows 는 Z-order 순이므로 첫 창이 최근 창
    }
}
```

테스트

```rust
fn w(h: isize, exe: &str) -> WinInfo { WinInfo { hwnd: h, exe: exe.into() } }

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
```

`open_url`

```rust
pub fn open_url(app: &AppHandle, url: &str, browser: &OsPath) -> anyhow::Result<()> {
    match browser.current() {
        Some(exe) => { std::process::Command::new(exe).arg(url).spawn()?; }
        None => app.opener().open_url(url, None::<&str>)?,
    }
    Ok(())
}
```

---

### [ ] Task 5. 전역 단축키 정규화·충돌 검사·등록·디스패치와 설정 저장

```text
commands::save_settings()                          [NEW]
  settings::validate()                             [NEW]
    hotkey::registry::collect_bindings()           [NEW]
      hotkey::accelerator::normalize()             [NEW]
    hotkey::registry::find_conflicts()             [NEW]   // 중복 → Err(ValidationError)
  SettingsStore#save()                             [Task 2]
  lib.rs#apply_settings()                          [NEW]
    hotkey::registry::HotkeyManager#reload()       [NEW]
      GlobalShortcut#unregister_all() / register() // plugin boundary, 실패 → failures
    tauri_plugin_autostart::ManagerExt#autolaunch()// enable / disable
lib.rs#run() setup                                 [MOD]
  lib.rs#apply_settings()                          [NEW]
  tray::notify_hotkey_failures()                   [NEW]   // 실패 항목 있을 때
global-shortcut handler                            [NEW]
  HotkeyManager#dispatch()                         [NEW]
    launcher::run()                                [Task 4]
    window_layout::apply()                         [Task 3]
commands::set_hotkeys_paused()                     [NEW]
commands::get_hotkey_failures()                    [NEW]
```

- [ ] 테스트 먼저: `hotkey/accelerator.rs`, `hotkey/registry.rs` 하단 `#[cfg(test)]`
- [ ] 의존성: `tauri-plugin-global-shortcut = "2"`, `tauri-plugin-autostart = "2"`, `tauri-plugin-notification = "2"`, `thiserror`
- [ ] `accelerator::normalize(&str) -> Result<String, AccelError>`: 대소문자·별칭(`Control`→`Ctrl`, `Option`→`Alt`, `Win`/`Cmd`/`Meta`→`Super`) 정리, 모디파이어 `Ctrl, Alt, Shift, Super` 순, 키 정확히 1개, 모디파이어 최소 1개(F1–F24 는 단독 허용)
- [ ] `Action` 열거형은 이 단위에서 `LaunchItem(String) | Layout(LayoutCommand) | ClipboardPopup` 로 정의하고, `ClipboardPopup` 디스패치는 Task 7 에서 연결한다. 이 단위에서는 `ClipboardPopup` 바인딩을 수집·충돌 검사만 하고 등록 대상에서 제외한다
- [ ] 비활성 기능의 바인딩은 수집하지 않는다(충돌 검사·등록 모두 제외)
- [ ] `HotkeyManager { by_id: HashMap<u32, Action>, failures: Vec<HotkeyFailure>, paused: bool }` 를 `AppState` 에 추가
- [ ] 등록 실패는 `HotkeyFailure { source: BindingSource, hotkey, reason }` 로 기록하고 저장 결과로 반환, 시작 시 1개 이상이면 알림 "단축키 N개를 등록하지 못했습니다"
- [ ] `save_settings` 반환: `Result<SaveResult { hotkey_failures }, ValidationError>`. `ValidationError { field, message }` 목록을 프론트에 그대로 표시
- [ ] `set_hotkeys_paused(true)` 는 모두 해제, `false` 는 재등록(설정 창 단축키 녹화 중 사용)
- [ ] 성공 기준
  - [ ] 테스트 통과
  - [ ] 표기만 다른 같은 단축키를 충돌로 판단
  - [ ] 비활성 기능 단축키는 충돌 대상 아님
  - [ ] 수동: 창 배치 단축키 16개, 실행 단축키(앱·URL) 동작
  - [ ] 수동: 다른 앱이 선점한 단축키 저장 시 실패 목록 반환, 재시작 시 알림

`src-tauri/src/hotkey/accelerator.rs`

```rust
const MOD_ORDER: [&str; 4] = ["Ctrl", "Alt", "Shift", "Super"];

pub fn normalize(input: &str) -> Result<String, AccelError> {
    let mut mods = [false; 4];
    let mut key: Option<String> = None;
    for raw in input.split('+').map(str::trim).filter(|s| !s.is_empty()) {
        let m = match raw.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => Some(0),
            "alt" | "option" => Some(1),
            "shift" => Some(2),
            "super" | "win" | "cmd" | "command" | "meta" => Some(3),
            _ => None,
        };
        match m {
            Some(i) => mods[i] = true,
            None if key.is_none() => key = Some(canonical_code(raw)?),
            None => return Err(AccelError::MultipleKeys),
        }
    }
    let key = key.ok_or(AccelError::NoKey)?;
    // F키만 모디파이어 없이 전역 등록을 허용한다 (일반 키 단독은 타이핑을 가로챈다)
    if !mods.iter().any(|m| *m) && !is_function_key(&key) {
        return Err(AccelError::NoModifier);
    }
    let mut parts: Vec<&str> = MOD_ORDER.iter().zip(mods).filter(|(_, on)| *on).map(|(n, _)| *n).collect();
    parts.push(&key);
    Ok(parts.join("+"))
}

/// "u" → "KeyU", "1" → "Digit1", "left" → "ArrowLeft", "return" → "Enter", 이미 Code 이름이면 대소문자만 정리.
fn canonical_code(raw: &str) -> Result<String, AccelError> { /* 표 기반 매핑 */ }
```

`src-tauri/src/hotkey/registry.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "camelCase")]
pub enum BindingSource { Launch(String), Window(LayoutCommand), Clipboard, Macro(usize) }

pub struct Binding { pub source: BindingSource, pub accel: String }

/// 같은 정규화 문자열을 가진 바인딩 묶음(2개 이상)을 돌려준다.
pub fn find_conflicts(bindings: &[Binding]) -> Vec<Vec<BindingSource>> {
    let mut map: BTreeMap<&str, Vec<BindingSource>> = BTreeMap::new();
    for b in bindings { map.entry(&b.accel).or_default().push(b.source.clone()); }
    map.into_values().filter(|v| v.len() > 1).collect()
}
```

`Macro(usize)` 는 Task 11 의 매크로 트리거가 전역 단축키와 겹치지 않도록 검사하기 위해 이 단위에서 수집한다(`settings.remap.rules` 의 `Macro { trigger }` 인덱스).

테스트

```rust
#[test]
fn normalize_orders_and_aliases() {
    assert_eq!(normalize("shift+ctrl+v").unwrap(), "Ctrl+Shift+KeyV");
    assert_eq!(normalize("Win + Alt + Control + Right").unwrap(), "Ctrl+Alt+Super+ArrowRight");
    assert_eq!(normalize("F5").unwrap(), "F5");
}

#[test]
fn normalize_rejects_bad_input() {
    assert_eq!(normalize("A"), Err(AccelError::NoModifier));
    assert_eq!(normalize("Ctrl+A+B"), Err(AccelError::MultipleKeys));
    assert_eq!(normalize("Ctrl+Shift"), Err(AccelError::NoKey));
}

#[test]
fn conflict_detected_across_features() {
    let mut s = Settings::default();
    s.launch_items.push(LaunchItem {
        id: "slack".into(), name: "Slack".into(), hotkey: "alt+ctrl+left".into(),
        target: LaunchTarget::Url { url: "https://x".into(), browser: OsPath::default() },
    });
    let conflicts = find_conflicts(&collect_bindings(&s).unwrap());
    assert_eq!(conflicts, vec![vec![
        BindingSource::Launch("slack".into()),
        BindingSource::Window(LayoutCommand::LeftHalf),
    ]].into_iter().map(sorted).collect::<Vec<_>>());
}

#[test]
fn disabled_feature_is_not_collected() {
    let mut s = Settings::default();
    s.general.features.window = false;
    assert!(collect_bindings(&s).unwrap().iter().all(|b| !matches!(b.source, BindingSource::Window(_))));
}
```

`collect_bindings` 는 출처 순서를 `Launch → Window → Clipboard → Macro` 로 넣고, `find_conflicts` 결과 묶음 안은 이 순서를 유지한다(테스트의 `sorted` 는 그 순서로 정렬하는 헬퍼).

---

### [ ] Task 6. 클립보드 저장소와 변경 감시

```text
lib.rs#run() setup                                 [MOD]
  clipboard::store::ClipStore#open()               [NEW]
    rusqlite::Connection#open()                    // DB boundary
  clipboard::watcher::start()                      [NEW]
    clipboard_rs::ClipboardWatcherContext#start_watch()  // crate boundary
watcher 변경 콜백                                    [NEW]
  clipboard::watcher::read_current()               [NEW]   // files > image > text 순
  ClipStore#record()                               [NEW]
    ClipStore#prune()                              [NEW]
lib.rs#apply_settings()                            [MOD]
  ClipStore#prune()                                [NEW]   // max_items 변경 반영
commands::list_clips() / set_pinned() / delete_clip() / clear_clips()  [NEW]
```

- [ ] 테스트 먼저: `clipboard/store.rs` 하단 `#[cfg(test)]` (in-memory DB + 임시 디렉터리)
- [ ] 의존성: `rusqlite = { version = "0.40", features = ["bundled"] }`, `clipboard-rs = "0.3"`, `image = "0.25"`, `sha2 = "0.11"`, `base64 = "0.22"`, `uuid = { version = "1", features = ["v4"] }`, 테스트용 `tempfile`
- [ ] 위치: `app_data_dir()/clipboard.db`, 이미지 `app_data_dir()/clips/{uuid}.png`, 썸네일 `{uuid}.thumb.png`(긴 변 160px)
- [ ] 동일 내용 판정: `sha256(kind + 내용)`. 이미지는 RGBA 원본 바이트로 해시
- [ ] 이미지 PNG 크기가 `max_image_mb` 초과면 기록하지 않음
- [ ] 클립보드 기능 꺼짐이면 콜백에서 기록하지 않음. `suppress_until: Mutex<Option<Instant>>` 이전 변경도 무시(Task 10 스니펫 임시 복사용, 이 단위에서 필드와 검사 로직까지 만든다)
- [ ] 목록 조회는 최대 200개, `query` 가 있으면 텍스트·파일 항목만 `LIKE` 검색(`%`, `_`, `\` 이스케이프)
- [ ] 삭제 시 이미지·썸네일 파일도 삭제
- [ ] 성공 기준
  - [ ] 테스트 통과
  - [ ] 같은 내용 재복사 시 행 수 그대로, 맨 위로 이동
  - [ ] 고정 항목은 개수 초과 정리·전체 삭제에서 유지
  - [ ] 수동: 텍스트/스크린샷/탐색기 파일 복사가 DB 에 기록

`src-tauri/src/clipboard/store.rs`

```rust
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS clips (
  id INTEGER PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('text','image','files')),
  text TEXT,                 -- text: 본문, files: 경로를 \n 으로 연결, image: NULL
  image_file TEXT,           -- image: uuid (확장자 제외)
  hash TEXT NOT NULL UNIQUE,
  pinned INTEGER NOT NULL DEFAULT 0,
  last_used_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_clips_last_used ON clips(last_used_at DESC);";

pub enum ClipContent { Text(String), Files(Vec<String>), Image { png: Vec<u8>, thumb_png: Vec<u8>, rgba_hash: String } }

impl ClipStore {
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

    /// 고정되지 않은 항목을 최신순 max_items 개만 남긴다.
    pub fn prune(&mut self, max_items: u32) -> rusqlite::Result<()> {
        let victims: Vec<(i64, Option<String>)> = self.conn
            .prepare("SELECT id, image_file FROM clips WHERE pinned = 0 ORDER BY last_used_at DESC LIMIT -1 OFFSET ?1")?
            .query_map([max_items], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        for (id, image) in victims { self.delete_row(id, image.as_deref())?; }
        Ok(())
    }
}
```

테스트

```rust
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
    for i in 2..=5 { s.record(ClipContent::Text(format!("t{i}")), i, 2).unwrap(); }
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
```

---

### [ ] Task 7. 클립보드 팝업과 직전 창 붙여넣기

```text
HotkeyManager#dispatch()                           [MOD]
  clipboard::popup::show()                         [NEW]   // Action::ClipboardPopup
    platform::current::window_ops::foreground()    [Task 3]  // 직전 창 저장
    platform::current::window_ops::monitor_at_cursor()  [NEW]
    WebviewWindow#set_position() / show() / set_focus()  // framework boundary
    Emitter#emit("clipboard-popup-opened")
commands::list_clips()                             [Task 6]
commands::paste_clip()                             [NEW]
  clipboard::popup::paste()                        [NEW]
    ClipStore#get()                                [NEW]
    clipboard_rs::Clipboard#set_text/set_image/set_files  // crate boundary
    WebviewWindow#hide()
    platform::current::process::focus()            [Task 4]
    platform::current::input::send_paste()         [NEW]
      SendInput                                    // Win32 boundary
commands::hide_clipboard_popup()                   [NEW]
```

- [ ] `HotkeyManager` 등록 대상에 `ClipboardPopup` 포함(Task 5 에서 제외해 두었던 것), 클립보드 기능 꺼짐이면 미등록
- [ ] `platform/windows/input.rs`: `send_keys(&[KeyCombo])` 와 `send_paste()`(Ctrl+V). 모든 주입 이벤트의 `dwExtraInfo` 에 `INJECTED_MARK` 를 넣는다(Task 9 훅이 자기 주입을 무시하는 기준)
- [ ] 붙여넣기 순서: 클립보드 쓰기 → 팝업 숨김 → 직전 창 포커스 → 50ms 대기 → Ctrl+V. 선택 항목은 감시자가 같은 hash 로 받아 맨 위로 올라간다
- [ ] 팝업은 포커스를 잃으면 숨김(`WindowEvent::Focused(false)`)
- [ ] `src/popup/ClipboardPopup.tsx`: 검색 입력(열릴 때마다 초기화·포커스), 목록(텍스트 1줄 미리보기, 이미지 썸네일, 파일은 첫 경로 + "외 N개"), ↑/↓ 선택, Enter 붙여넣기, Esc 숨김, 항목별 핀 토글·삭제 버튼. 검색은 입력 150ms 디바운스 후 `list_clips(query)`
- [ ] 성공 기준
  - [ ] `npm run build`, `cargo test` 통과
  - [ ] 수동: Ctrl+Shift+V → 커서가 있는 모니터 중앙 표시
  - [ ] 수동: 메모장에서 텍스트·이미지(그림판)·파일(탐색기) 붙여넣기 성공
  - [ ] 수동: 팝업 밖 클릭 시 닫힘

`src-tauri/src/clipboard/popup.rs`

```rust
pub fn show(app: &AppHandle) -> anyhow::Result<()> {
    let state = app.state::<AppState>();
    *state.popup_prev.lock().unwrap() = window_ops::foreground();
    let win = app.get_webview_window("clipboard-popup").context("popup window")?;
    let work = window_ops::monitor_at_cursor()?;
    let size = win.outer_size()?;
    let x = work.x + (work.w - size.width as i32) / 2;
    let y = work.y + (work.h - size.height as i32) / 2;
    win.set_position(tauri::PhysicalPosition::new(x, y))?;
    win.show()?;
    win.set_focus()?;
    app.emit_to("clipboard-popup", "clipboard-popup-opened", ())?;
    Ok(())
}

pub fn paste(app: &AppHandle, id: i64) -> anyhow::Result<()> {
    let state = app.state::<AppState>();
    let clip = state.clips.lock().unwrap().get(id)?;
    write_to_clipboard(&clip)?;
    app.get_webview_window("clipboard-popup").context("popup window")?.hide()?;
    if let Some(prev) = *state.popup_prev.lock().unwrap() {
        process::focus(prev)?;
    }
    // 포커스 전환이 끝나기 전에 Ctrl+V 가 가면 팝업/이전 창이 아닌 곳에 입력된다
    std::thread::sleep(std::time::Duration::from_millis(50));
    input::send_paste()
}
```

`src/popup/ClipboardPopup.tsx` 키 처리

```tsx
function onKeyDown(e: React.KeyboardEvent) {
  if (e.key === "ArrowDown") { e.preventDefault(); setSel(i => Math.min(i + 1, clips.length - 1)); }
  else if (e.key === "ArrowUp") { e.preventDefault(); setSel(i => Math.max(i - 1, 0)); }
  else if (e.key === "Enter" && !e.nativeEvent.isComposing && clips[sel]) { invoke("paste_clip", { id: clips[sel].id }); }
  else if (e.key === "Escape") { invoke("hide_clipboard_popup"); }
}
```

`isComposing` 검사는 한글 검색어 입력 중 Enter 가 조합 확정과 붙여넣기를 동시에 일으키지 않게 한다.

---

### [ ] Task 8. 설정 창 UI (일반·실행 단축키·창 조절·클립보드)

```text
SettingsApp                                   [NEW]
  invoke("get_settings")                      [Task 2]
  invoke("get_hotkey_failures")               [Task 5]
  GeneralTab / LaunchTab / WindowTab / ClipboardTab   [NEW]
    HotkeyInput                               [NEW]
      toAccelerator()                         [NEW]
      invoke("set_hotkeys_paused")            [Task 5]   // focus / blur
    invoke("list_installed_apps")             [Task 4]   // LaunchTab
    dialog.open()                             // plugin boundary: exe/브라우저/가져오기 파일 선택
  invoke("save_settings")                     [Task 5]
    ValidationError → 필드 옆 메시지 / SaveResult.hotkeyFailures → 항목 오류 배지
GeneralTab
  invoke("export_settings") / invoke("import_settings")  [Task 2]
```

- [ ] 테스트 먼저: `src/lib/accelerator.test.ts` (vitest). `package.json` 에 `vitest` devDependency, `"test": "vitest run"`
- [ ] 의존성: `@tauri-apps/plugin-dialog`, Rust `tauri-plugin-dialog = "2"`, capabilities 에 `dialog:default`
- [ ] `src/api.ts`: Rust 모델과 같은 이름의 TS 타입(`Settings`, `LaunchItem`, `LaunchTarget`, `ClipItem`, `SaveResult`, `ValidationError`)과 command 래퍼
- [ ] 편집은 로컬 draft 상태에서 하고 "저장" 버튼으로 `save_settings`. 저장 실패 시 draft 유지
- [ ] LaunchTab: 이름, 대상 종류(앱/URL), 앱은 설치 앱 검색 드롭다운 + "파일 선택"(exe/lnk), URL 은 주소 + 브라우저(선택, exe 파일 선택·지우기), 단축키. 현재 OS 의 경로 필드만 편집하고 다른 OS 값은 보존
- [ ] WindowTab: 16개 명령 이름(한글)과 단축키
- [ ] ClipboardTab: 팝업 단축키, 최대 개수(1–10000), 이미지 최대 MB(1–100), "전체 삭제(고정 제외)" 버튼 → `clear_clips`
- [ ] GeneralTab: 자동 실행, 기능 5개 토글, 내보내기/가져오기. 가져오기 후 화면 새로고침
- [ ] 성공 기준
  - [ ] `npm test`, `npm run build` 통과
  - [ ] 수동: 각 탭 저장 후 재시작해도 유지
  - [ ] 수동: 중복 단축키 저장 차단 메시지, OS 선점 단축키 오류 배지
  - [ ] 수동: 단축키 녹화 중 Ctrl+Alt+← 를 눌러도 창이 이동하지 않음

`src/lib/accelerator.ts`

```ts
const MODIFIER_CODES = new Set([
  "ControlLeft", "ControlRight", "AltLeft", "AltRight",
  "ShiftLeft", "ShiftRight", "MetaLeft", "MetaRight",
]);

/** 모디파이어만 눌린 상태면 null (녹화 계속). Rust accelerator::normalize 와 같은 순서를 만든다. */
export function toAccelerator(e: Pick<KeyboardEvent, "ctrlKey" | "altKey" | "shiftKey" | "metaKey" | "code">): string | null {
  if (MODIFIER_CODES.has(e.code)) return null;
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Super");
  parts.push(e.code);
  return parts.join("+");
}
```

`src/lib/accelerator.test.ts`

```ts
import { describe, expect, it } from "vitest";
import { toAccelerator } from "./accelerator";

const ev = (code: string, m: Partial<Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>> = {}) =>
  ({ ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, code, ...m });

describe("toAccelerator", () => {
  it("orders modifiers like the Rust normalizer", () => {
    expect(toAccelerator(ev("ArrowRight", { metaKey: true, altKey: true, ctrlKey: true }))).toBe("Ctrl+Alt+Super+ArrowRight");
  });
  it("waits while only modifiers are pressed", () => {
    expect(toAccelerator(ev("ShiftLeft", { shiftKey: true }))).toBeNull();
  });
  it("uses physical code so Korean IME input still yields KeyV", () => {
    expect(toAccelerator(ev("KeyV", { ctrlKey: true, shiftKey: true }))).toBe("Ctrl+Shift+KeyV");
  });
});
```

`src/components/HotkeyInput.tsx`

```tsx
export function HotkeyInput({ value, onChange, error }: Props) {
  const [recording, setRecording] = useState(false);
  const start = () => { setRecording(true); invoke("set_hotkeys_paused", { paused: true }); };
  const stop = () => { setRecording(false); invoke("set_hotkeys_paused", { paused: false }); };
  return (
    <input
      readOnly
      className={error ? "hotkey error" : "hotkey"}
      value={recording ? "키를 누르세요…" : value}
      onFocus={start}
      onBlur={stop}
      onKeyDown={e => {
        e.preventDefault();
        if (e.key === "Escape") { (e.target as HTMLInputElement).blur(); return; }
        const accel = toAccelerator(e.nativeEvent);
        if (accel) { onChange(accel); (e.target as HTMLInputElement).blur(); }
      }}
      title={error}
    />
  );
}
```

---

### [ ] Task 9. 전역 키보드 훅 모듈

```text
lib.rs#run() setup                                [MOD]
  keyboard::start()                               [NEW]
    platform::current::hook::install()            [NEW]   // 전용 스레드 + 메시지 루프
      SetWindowsHookExW(WH_KEYBOARD_LL)           // Win32 boundary
hook 콜백                                          [NEW]
  platform::current::hook::to_event()             [NEW]
    keyboard::keys::vk_to_code()                  [NEW]
  keyboard::Dispatcher#handle()                   [NEW]
    KeyHandler#on_key()                           [NEW]   // 등록된 핸들러 순서대로, Suppress 면 중단
  CallNextHookEx                                  // Pass
```

- [ ] 테스트 먼저: `keyboard/keys.rs`(Code↔VK 왕복), `keyboard/mod.rs`(Dispatcher 순서·중단)
- [ ] `KeyEvent { code: String, down: bool, time_ms: u64, injected: bool, ch: Option<char> }`. `ch` 는 Task 10 매처용으로 훅에서 `ToUnicodeEx(.., wFlags = 0x4)` 로 계산(키보드 상태를 바꾸지 않아 데드키·IME 에 영향 없음). `VK_PROCESSKEY`(IME 처리 중) 이면 `code = "Process"`, `ch = None`
- [ ] `injected` 는 `dwExtraInfo == INJECTED_MARK` 일 때만 true. 다른 프로그램의 주입 입력은 일반 입력으로 취급
- [ ] `trait KeyHandler: Send { fn on_key(&mut self, ev: &KeyEvent) -> KeyDecision; }`, `KeyDecision::{Pass, Suppress}`
- [ ] 훅 콜백은 빠르게 반환해야 하므로(OS 타임아웃 시 훅이 해제된다) 핸들러 안에서 SendInput 외 블로킹 작업 금지. 붙여넣기처럼 대기가 필요한 동작은 별도 스레드로 보낸다
- [ ] 이 단위의 Dispatcher 핸들러 목록은 비어 있다(Task 10, 11 이 추가)
- [ ] 성공 기준
  - [ ] 테스트 통과
  - [ ] 수동: 앱 실행 중 타이핑 지연·누락 없음

`src-tauri/src/keyboard/mod.rs`

```rust
pub struct Dispatcher { handlers: Vec<Box<dyn KeyHandler>> }

impl Dispatcher {
    pub fn handle(&mut self, ev: &KeyEvent) -> KeyDecision {
        // 자기 주입 이벤트는 핸들러에 다시 넣지 않는다 (리매핑 출력이 재리매핑되는 루프 방지)
        if ev.injected { return KeyDecision::Pass; }
        for h in &mut self.handlers {
            if h.on_key(ev) == KeyDecision::Suppress { return KeyDecision::Suppress; }
        }
        KeyDecision::Pass
    }
}
```

테스트

```rust
struct Rec(Arc<Mutex<Vec<&'static str>>>, &'static str, KeyDecision);
impl KeyHandler for Rec {
    fn on_key(&mut self, _: &KeyEvent) -> KeyDecision { self.0.lock().unwrap().push(self.1); self.2 }
}

#[test]
fn suppress_stops_chain_and_injected_skips_all() {
    let log = Arc::new(Mutex::new(vec![]));
    let mut d = Dispatcher::new(vec![
        Box::new(Rec(log.clone(), "remap", KeyDecision::Suppress)),
        Box::new(Rec(log.clone(), "snippet", KeyDecision::Pass)),
    ]);
    assert_eq!(d.handle(&key("KeyA", true)), KeyDecision::Suppress);
    assert_eq!(d.handle(&KeyEvent { injected: true, ..key("KeyA", true) }), KeyDecision::Pass);
    assert_eq!(*log.lock().unwrap(), vec!["remap"]);
}

#[test]
fn vk_code_roundtrip() {
    for code in ["KeyA", "Digit0", "ArrowLeft", "CapsLock", "Escape", "Enter", "F12", "Semicolon", "ShiftLeft"] {
        assert_eq!(vk_to_code(code_to_vk(code).unwrap()).as_deref(), Some(code));
    }
}
```

---

### [ ] Task 10. 텍스트 스니펫 자동 확장

```text
keyboard::Dispatcher#handle()                     [Task 9]
  snippet::SnippetHandler#on_key()                [NEW]
    snippet::Matcher#feed()                       [NEW]
    snippet::expand()                             [NEW]   // 매치 시 별도 스레드
      ClipboardWatcher suppress_until 설정         [Task 6]
      clipboard_rs::Clipboard get/set             // crate boundary
      platform::current::input::send_keys()       [Task 7]  // Backspace × (len-1)
      platform::current::input::send_paste()      [Task 7]
settings::validate()                              [MOD]   // 키워드 규칙
lib.rs#apply_settings()                           [MOD]   // Matcher 키워드 갱신
SnippetTab                                        [NEW]
```

- [ ] 테스트 먼저: `snippet.rs` 하단 `#[cfg(test)]`
- [ ] 키워드 검증: 비어 있지 않음, 2–32자, ASCII 출력 가능 문자만(공백 제외), 중복 금지, 다른 키워드의 접두어 금지
- [ ] 매처 버퍼 초기화 조건: `ch` 없는 키 down(화살표, Enter, Tab, Esc, `Process` 등. 모디파이어 단독은 제외), Ctrl/Alt/Super 와 함께 눌린 키, 포그라운드 창 변경(훅에서 직전 hwnd 비교)
- [ ] Backspace 는 버퍼 마지막 문자 제거
- [ ] 매치 시 마지막 키를 Suppress 하고(입력되지 않음), 이미 입력된 `len-1` 글자를 Backspace 로 지운 뒤 텍스트 붙여넣기, 원래 클립보드 복원. 복원까지 클립보드 기록 억제(`suppress_until = now + 1s`)
- [ ] 스니펫 기능 꺼짐이면 항상 Pass
- [ ] SnippetTab: 키워드·텍스트(여러 줄) 목록 편집, 검증 오류 표시
- [ ] 성공 기준
  - [ ] 테스트 통과
  - [ ] 즉시 치환(구분자 불필요), 가장 긴 키워드 우선
  - [ ] 접두어 관계 키워드 저장 차단
  - [ ] 수동: 메모장·브라우저 입력창에서 `;addr` 치환, 원래 클립보드 유지, 히스토리에 임시 항목 없음
  - [ ] 수동: 한글 입력 모드에서는 치환되지 않음

`src-tauri/src/snippet.rs`

```rust
pub struct Matcher { keywords: Vec<(String, String)>, buf: String }

pub enum Feed { Pass, Expand { erase: usize, text: String } }

impl Matcher {
    const MAX_BUF: usize = 32;

    pub fn feed(&mut self, ev: &KeyEvent, has_cmd_mod: bool) -> Feed {
        if !ev.down || is_modifier(&ev.code) { return Feed::Pass; }
        if ev.code == "Backspace" { self.buf.pop(); return Feed::Pass; }
        let Some(ch) = ev.ch.filter(|c| !has_cmd_mod && !c.is_control()) else {
            self.buf.clear();
            return Feed::Pass;
        };
        self.buf.push(ch);
        if self.buf.len() > Self::MAX_BUF { self.buf.drain(..self.buf.len() - Self::MAX_BUF); }
        let hit = self.keywords.iter()
            .filter(|(k, _)| self.buf.ends_with(k.as_str()))
            .max_by_key(|(k, _)| k.len());
        match hit {
            Some((k, text)) => {
                let erase = k.chars().count() - 1; // 마지막 글자는 Suppress 되어 입력되지 않는다
                let text = text.clone();
                self.buf.clear();
                Feed::Expand { erase, text }
            }
            None => Feed::Pass,
        }
    }
}

pub fn validate_keywords(snippets: &[Snippet]) -> Vec<ValidationError> { /* 규칙은 위 체크리스트 */ }
```

테스트

```rust
fn typed(m: &mut Matcher, s: &str) -> Vec<Feed> {
    s.chars().map(|c| m.feed(&char_event(c), false)).collect()
}

#[test]
fn expands_immediately_on_last_char() {
    let mut m = Matcher::new(vec![(";addr".into(), "서울시".into())]);
    let out = typed(&mut m, "x;addr");
    assert!(matches!(out.last(), Some(Feed::Expand { erase: 4, text }) if text == "서울시"));
}

#[test]
fn longest_keyword_wins() {
    let mut m = Matcher::new(vec![("dr".into(), "A".into()), (";addr".into(), "B".into())]);
    assert!(matches!(typed(&mut m, ";addr").last(), Some(Feed::Expand { text, .. }) if text == "B"));
}

#[test]
fn backspace_and_navigation_affect_buffer() {
    let mut m = Matcher::new(vec![(";ab".into(), "X".into())]);
    typed(&mut m, ";a");
    m.feed(&key("Backspace", true), false);
    assert!(matches!(typed(&mut m, "ab").last(), Some(Feed::Pass)));
    typed(&mut m, ";a");
    m.feed(&key("ArrowLeft", true), false);
    assert!(matches!(typed(&mut m, "b").last(), Some(Feed::Pass)));
}

#[test]
fn ime_process_key_resets_buffer() {
    let mut m = Matcher::new(vec![(";ab".into(), "X".into())]);
    typed(&mut m, ";a");
    m.feed(&key("Process", true), false);
    assert!(matches!(typed(&mut m, "b").last(), Some(Feed::Pass)));
}

#[test]
fn prefix_keywords_rejected() {
    let errs = validate_keywords(&[snippet(";a", "x"), snippet(";ab", "y")]);
    assert_eq!(errs.len(), 1);
}
```

---

### [ ] Task 11. 키 리매핑 엔진과 제외 앱

```text
keyboard::Dispatcher#handle()                     [Task 9]
  remap::RemapHandler#on_key()                    [NEW]
    platform::current::process::foreground_exe()  [NEW]   // 제외 앱 → Pass + 상태 초기화
    remap::Engine#process()                       [NEW]
    platform::current::input::send_events()       [NEW]   // Engine 출력 주입
settings::validate()                              [MOD]   // 규칙 키 이름·중복 트리거
lib.rs#apply_settings()                           [MOD]   // Engine 규칙 갱신
RemapTab                                          [NEW]
```

- [ ] 테스트 먼저: `remap.rs` 하단 `#[cfg(test)]`
- [ ] 엔진은 순수 함수형 상태 기계: `process(&mut self, ev: &KeyEvent) -> Output { suppress: bool, emit: Vec<(String, bool)> }` (`(code, down)`). OS 호출 없음
- [ ] 우선순위: 진행 중인 TapHold/Layer 상태 → Macro 트리거(모디파이어 상태 포함 일치) → Simple → TapHold/Layer 시작 → Pass
- [ ] `KeyCombo` 출력: down 시 mods 순서대로 down 후 key down, up 시 key up 후 mods 역순 up. 탭 출력은 down+up 을 연속으로
- [ ] 엔진이 내부적으로 눌림을 추적하는 모디파이어(Ctrl/Alt/Shift/Super)는 물리 키 기준(`injected=false`)
- [ ] 검증: 키 이름은 `code_to_vk` 로 변환 가능해야 함, 같은 키를 트리거로 쓰는 규칙(Simple.from, TapHold.key, Layer.trigger) 중복 금지, Macro.trigger 는 `accelerator::normalize` 통과 + 전역 단축키와 충돌 금지(Task 5 의 `BindingSource::Macro`)
- [ ] 제외 앱: `foreground_exe()` 의 파일 이름을 소문자로 비교. 포그라운드가 제외 앱으로 바뀌면 엔진 `reset()` 후 눌린 출력 키를 모두 up 으로 보낸다(키 고착 방지)
- [ ] 리매핑 기능 꺼짐이면 항상 Pass
- [ ] RemapTab: 규칙 유형 선택(단일 키 / 탭·홀드 / 레이어 / 매크로) 별 입력 폼, 키 이름은 키 녹화(물리 code), 제외 앱 목록(파일 이름 입력 + 실행 중 앱 선택은 하지 않음)
- [ ] 성공 기준
  - [ ] 테스트 통과
  - [ ] 단일 키, Hyper, 레이어, 탭/홀드, 매크로 각각 출력 순서 정확
  - [ ] 제외 앱 전환 시 눌린 키 해제
  - [ ] 수동: CapsLock→Esc, CapsLock 홀드+HJKL 방향키, CapsLock 탭 Esc/홀드 Ctrl, 매크로 실행
  - [ ] 수동: 제외 앱(예: 메모장 지정)에서 리매핑 비활성

`src-tauri/src/remap.rs`

```rust
const TAP_TIMEOUT_MS: u64 = 200;

enum Pending {
    TapHold { key: String, tap: KeyCombo, hold: KeyCombo, since: u64, held: bool },
    Layer { trigger: String, tap: Option<KeyCombo>, since: u64, used: bool },
}

impl Engine {
    pub fn process(&mut self, ev: &KeyEvent) -> Output {
        if let Some(out) = self.continue_pending(ev) { return out; }
        if ev.down {
            if let Some(steps) = self.match_macro(ev) { self.swallow_up.insert(ev.code.clone()); return Output::suppress(press_all(steps)); }
        } else if self.swallow_up.remove(&ev.code) {
            return Output::suppress(vec![]);
        }
        if let Some(to) = self.simple.get(&ev.code) { return Output::suppress(combo_edge(to, ev.down)); }
        if ev.down { if let Some(p) = self.start_pending(ev) { self.pending = Some(p); return Output::suppress(vec![]); } }
        Output::pass()
    }

    /// TapHold 진행 중 처리: 다른 키 down → hold 확정 후 그 키는 그대로 통과,
    /// 같은 키 up → 200ms 미만·미확정이면 tap, 확정됐으면 hold up, 그 외는 출력 없음.
    fn continue_pending(&mut self, ev: &KeyEvent) -> Option<Output> { /* ... */ }
}
```

테스트

```rust
fn caps_tap_hold() -> Engine {
    Engine::new(&[RemapRule::TapHold {
        key: "CapsLock".into(),
        tap: KeyCombo { mods: vec![], key: Some("Escape".into()) },
        hold: KeyCombo { mods: vec!["Ctrl".into()], key: None },
    }])
}

#[test]
fn simple_maps_down_and_up() {
    let mut e = Engine::new(&[RemapRule::Simple { from: "CapsLock".into(), to: combo(&[], "Escape") }]);
    assert_eq!(e.process(&at("CapsLock", true, 0)), Output::suppress(vec![("Escape".into(), true)]));
    assert_eq!(e.process(&at("CapsLock", false, 10)), Output::suppress(vec![("Escape".into(), false)]));
}

#[test]
fn hyper_key_presses_all_modifiers_in_order() {
    let mut e = Engine::new(&[RemapRule::Simple { from: "CapsLock".into(), to: KeyCombo { mods: mods(&["Ctrl", "Alt", "Shift", "Super"]), key: None } }]);
    let down = e.process(&at("CapsLock", true, 0)).emit;
    assert_eq!(down, vec![("ControlLeft".into(), true), ("AltLeft".into(), true), ("ShiftLeft".into(), true), ("MetaLeft".into(), true)]);
    let up = e.process(&at("CapsLock", false, 5)).emit;
    assert_eq!(up.first(), Some(&("MetaLeft".into(), false)));
}

#[test]
fn tap_within_timeout_sends_tap() {
    let mut e = caps_tap_hold();
    assert!(e.process(&at("CapsLock", true, 0)).emit.is_empty());
    assert_eq!(e.process(&at("CapsLock", false, 120)).emit, vec![("Escape".into(), true), ("Escape".into(), false)]);
}

#[test]
fn other_key_while_pending_resolves_hold() {
    let mut e = caps_tap_hold();
    e.process(&at("CapsLock", true, 0));
    let out = e.process(&at("KeyC", true, 50));
    assert!(!out.suppress, "KeyC 자체는 통과");
    assert_eq!(out.emit, vec![("ControlLeft".into(), true)]);
    e.process(&at("KeyC", false, 60));
    assert_eq!(e.process(&at("CapsLock", false, 300)).emit, vec![("ControlLeft".into(), false)]);
}

#[test]
fn long_press_alone_sends_nothing() {
    let mut e = caps_tap_hold();
    e.process(&at("CapsLock", true, 0));
    assert!(e.process(&at("CapsLock", false, 500)).emit.is_empty());
}

#[test]
fn layer_maps_keys_while_trigger_held() {
    let mut e = Engine::new(&[RemapRule::Layer {
        trigger: "CapsLock".into(),
        tap: Some(combo(&[], "Escape")),
        mappings: vec![LayerMapping { from: "KeyH".into(), to: combo(&[], "ArrowLeft") }],
    }]);
    e.process(&at("CapsLock", true, 0));
    assert_eq!(e.process(&at("KeyH", true, 30)), Output::suppress(vec![("ArrowLeft".into(), true)]));
    assert_eq!(e.process(&at("KeyH", false, 40)), Output::suppress(vec![("ArrowLeft".into(), false)]));
    assert!(e.process(&at("CapsLock", false, 60)).emit.is_empty(), "레이어를 썼으면 tap 안 보냄");
    assert!(!e.process(&at("KeyH", true, 70)).suppress, "트리거 뗀 뒤에는 원래 키");
}

#[test]
fn macro_sends_sequence_and_swallows_trigger_up() {
    let mut e = Engine::new(&[RemapRule::Macro {
        trigger: "Ctrl+Alt+KeyM".into(),
        steps: vec![combo(&["Ctrl"], "KeyA"), combo(&["Ctrl"], "KeyC")],
    }]);
    e.process(&mod_down("ControlLeft")); e.process(&mod_down("AltLeft"));
    let out = e.process(&at("KeyM", true, 0));
    assert!(out.suppress);
    assert_eq!(out.emit, vec![
        ("ControlLeft".into(), true), ("KeyA".into(), true), ("KeyA".into(), false), ("ControlLeft".into(), false),
        ("ControlLeft".into(), true), ("KeyC".into(), true), ("KeyC".into(), false), ("ControlLeft".into(), false),
    ]);
    assert!(e.process(&at("KeyM", false, 10)).suppress);
}
```

모디파이어 이름 → 출력 code: `Ctrl→ControlLeft`, `Alt→AltLeft`, `Shift→ShiftLeft`, `Super→MetaLeft`. 매크로는 사용자가 누르고 있는 물리 모디파이어(Ctrl+Alt) 위에 주입되므로, 주입 전 눌린 물리 모디파이어를 up 으로 보내고 매크로 후 다시 down 으로 복원한다(테스트는 이 앞뒤 복원 이벤트를 제외한 핵심 시퀀스만 검증하므로 `emit` 비교 전에 `strip_restore()` 헬퍼로 앞뒤 복원 이벤트를 제거한다).

---

## 최종 검증 체크리스트

- [ ] `cargo test` (src-tauri) 전체 통과
- [ ] `cargo clippy -- -D warnings` 통과
- [ ] `npm test`, `npm run build` 통과
- [ ] `npm run tauri build` 로 설치 파일 생성
- [ ] 수동(Windows): 요구사항 문서의 MVP 1–3, 후속 1–2 항목을 한 번씩 실제 동작으로 확인
- [ ] 수동: 관리자 권한으로 실행한 창(관리자 메모장)에서 단축키·붙여넣기·스니펫·리매핑 동작 여부를 확인하고 결과를 요구사항 문서 `기타 사항` 에 기록
- [ ] 수동: 설정 내보내기 → 기본값 상태에서 가져오기 → 동일 동작

## 위험과 대응

| 위험 | 대응 |
|---|---|
| UIPI: 관리자 권한 창에 SendInput·훅 입력이 막힘 | 최종 검증에서 확인 후 기록. 필요하면 "관리자 권한으로 실행" 옵션을 별도 요구사항으로 제안 |
| 백그라운드 프로세스의 `SetForegroundWindow` 거부 | 단축키 처리 직후 호출(포그라운드 권한 부여 시점), 실패 시 `AttachThreadInput` 재시도 |
| LL 훅 콜백이 느리면 OS 가 훅을 조용히 해제 | 콜백 내 블로킹 금지, 붙여넣기 대기는 별도 스레드 |
| 스니펫 붙여넣기 중 원래 클립보드 복원 타이밍 | 붙여넣기 후 150ms 대기 후 복원, 억제 구간 1초 |
| Ctrl+Alt 조합이 일부 키보드 배열에서 AltGr 로 해석 | 한국어/영문 배열은 해당 없음. 문제 시 단축키 수정으로 대응 |
| `clipboard-rs` 파일 목록·이미지 API 차이 | Task 6 구현 시 crate 문서로 확인, 부족하면 Win32 `CF_HDROP`/`CF_DIB` 직접 처리로 대체 |
| macOS 미구현 | `platform::unsupported` 가 `Err("unsupported")` 를 반환해 빌드만 유지. 포팅은 별도 계획 |
