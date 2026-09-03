//! Claude 구독 한도 — 1순위 경로(공식 statusline 훅). ARCHITECTURE §5.1.
//!
//! Claude Code(CLI)는 세션 중 `settings.json`의 `statusLine.command`를 실행하며 표준 입력으로
//! 상태 JSON을 준다. 그 안의 `rate_limits`가 `/usage`와 같은 값이다. 이 모듈은
//! ① 그 명령을 우리 훅으로 **감싸 설치**하고(기존 명령은 그대로 이어서 실행), ② 훅이 남긴
//! 파일을 주기적으로 읽어 `usage:update` 계약(ARCHITECTURE §4)으로 정규화해 창에 보낸다.
//!
//! 네트워크를 쓰지 않고 토큰도 읽지 않는다. 2순위 경로(옵트인 조회)는 M1b에서 붙인다.

use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    env, fs,
    path::PathBuf,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// statusline 값이 이 시간보다 오래되면 `stale`. Claude Code 세션이 닫히면 갱신이 멈춘다.
const STALE_AFTER_SECS: u64 = 300;
const POLL: Duration = Duration::from_secs(2);
const HOOK_SH: &str = include_str!("../hook/hook.sh");
/// 설치된 훅을 알아보는 표식. `settings.json`에 이 문자열이 있으면 우리 것이다.
const HOOK_MARK: &str = "usage-monitor/hook.sh";
const HOOK_CMD: &str = r#"sh "$HOME/.claude/usage-monitor/hook.sh""#;

#[derive(Serialize, Clone, Default)]
pub struct Window {
    pub used_pct: f64,
    pub resets_at: Option<String>,
}

/// 모델별 주간 창. statusline에는 없고 옵트인 조회(`oauth.rs`)에서만 온다.
#[derive(Serialize, Clone)]
pub struct ModelWindow {
    pub label: String,
    pub used_pct: f64,
    pub resets_at: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct Usage {
    pub source: &'static str,
    pub status: &'static str,
    pub fetched_at: Option<String>,
    pub five_hour: Option<Window>,
    pub seven_day: Option<Window>,
    pub model_window: Option<ModelWindow>,
}

impl Usage {
    fn empty(status: &'static str) -> Self {
        Self { source: "statusline", status, fetched_at: None, five_hour: None, seven_day: None, model_window: None }
    }
    pub fn empty_oauth(status: &'static str) -> Self {
        Self { source: "oauth", ..Self::empty(status) }
    }
}

// ---------- 경로 ----------

/// Claude Code 설정 폴더. `CLAUDE_CONFIG_DIR`가 있으면 그것을 따른다 (조사 §B.2: 도구는 이 변수를 존중해야 한다).
pub fn claude_dir() -> Option<PathBuf> {
    if let Ok(d) = env::var("CLAUDE_CONFIG_DIR") {
        if !d.trim().is_empty() {
            return Some(PathBuf::from(d));
        }
    }
    home().map(|h| h.join(".claude"))
}

fn home() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn hook_dir() -> Option<PathBuf> { claude_dir().map(|d| d.join("usage-monitor")) }
fn status_path() -> Option<PathBuf> { hook_dir().map(|d| d.join("status.json")) }
fn settings_path() -> Option<PathBuf> { claude_dir().map(|d| d.join("settings.json")) }

// ---------- 훅 설치·제거 ----------

pub fn is_installed() -> bool {
    settings_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .map(|s| s.contains(HOOK_MARK))
        .unwrap_or(false)
}

/// 훅을 설치한다. 기존 statusline 명령이 있으면 `next-command`에 옮겨 두고 훅이 이어서 실행하므로
/// 사용자의 statusline 표시는 그대로 유지된다. 설정 파일은 먼저 백업한다.
pub fn install() -> Result<(), String> {
    let dir = hook_dir().ok_or("홈 폴더를 찾지 못했습니다")?;
    let sp = settings_path().ok_or("설정 폴더를 찾지 못했습니다")?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // 줄바꿈은 LF로 — Windows에서도 sh가 읽는다.
    fs::write(dir.join("hook.sh"), HOOK_SH.replace("\r\n", "\n")).map_err(|e| e.to_string())?;

    let mut root: Value = match fs::read_to_string(&sp) {
        Ok(s) => {
            fs::write(sp.with_extension("json.usage-monitor-backup"), &s).map_err(|e| e.to_string())?;
            serde_json::from_str(&s).map_err(|e| format!("settings.json을 읽지 못했습니다: {e}"))?
        }
        Err(_) => json!({}),
    };
    if !root.is_object() {
        return Err("settings.json의 형식이 예상과 다릅니다".into());
    }

    let prev = root
        .get("statusLine")
        .and_then(|s| s.get("command"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();
    if !prev.contains(HOOK_MARK) {
        // 이어서 실행할 원래 명령 (없으면 빈 파일 → 훅이 아무것도 출력하지 않는다).
        fs::write(dir.join("next-command"), &prev).map_err(|e| e.to_string())?;
    }
    root["statusLine"] = json!({ "type": "command", "command": HOOK_CMD });
    write_json(&sp, &root)
}

/// 훅을 제거하고 원래 statusline 명령을 되돌린다.
pub fn uninstall() -> Result<(), String> {
    let sp = settings_path().ok_or("설정 폴더를 찾지 못했습니다")?;
    let dir = hook_dir().ok_or("홈 폴더를 찾지 못했습니다")?;
    let s = fs::read_to_string(&sp).map_err(|e| e.to_string())?;
    let mut root: Value = serde_json::from_str(&s).map_err(|e| e.to_string())?;
    let prev = fs::read_to_string(dir.join("next-command")).unwrap_or_default();
    if prev.trim().is_empty() {
        root.as_object_mut().map(|o| o.remove("statusLine"));
    } else {
        root["statusLine"] = json!({ "type": "command", "command": prev });
    }
    let _ = fs::remove_file(dir.join("status.json"));
    write_json(&sp, &root)
}

fn write_json(p: &PathBuf, v: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(v).map_err(|e| e.to_string())?;
    let tmp = p.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, p).map_err(|e| e.to_string())
}

// ---------- 읽기·정규화 ----------

fn iso(secs: i64) -> Option<String> {
    Utc.timestamp_opt(secs, 0).single().map(|t: DateTime<Utc>| t.to_rfc3339())
}

/// `rate_limits.<key>` 하나를 계약의 창 하나로 옮긴다. 값이 없거나 이미 지난 창은 `None`.
fn window(rl: &Value, key: &str) -> Option<Window> {
    let w = rl.get(key)?;
    let used = w.get("used_percentage").and_then(Value::as_f64)?;
    let resets = w.get("resets_at").and_then(Value::as_i64);
    Some(Window { used_pct: used, resets_at: resets.and_then(iso) })
}

pub fn read() -> Usage {
    if !is_installed() {
        return Usage::empty("no_source");
    }
    let Some(p) = status_path() else { return Usage::empty("no_source") };
    let Ok(meta) = fs::metadata(&p) else { return Usage::empty("waiting") };
    let Ok(text) = fs::read_to_string(&p) else { return Usage::empty("waiting") };
    let Ok(root) = serde_json::from_str::<Value>(&text) else { return Usage::empty("shape_changed") };

    let age = meta
        .modified()
        .ok()
        .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
        .and_then(|d| SystemTime::now().duration_since(UNIX_EPOCH).ok().map(|n| n.as_secs().saturating_sub(d.as_secs())))
        .unwrap_or(0);
    let fetched_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|n| iso(n.as_secs() as i64 - age as i64));

    // rate_limits는 Pro/Max에게만, 그리고 세션의 첫 응답 이후에만 나온다 (공식 문서).
    let Some(rl) = root.get("rate_limits") else { return Usage::empty("waiting") };
    let five = window(rl, "five_hour");
    if five.is_none() && window(rl, "seven_day").is_none() {
        // 키는 있는데 아는 창이 하나도 없다 — 형식이 바뀌었거나 아직 채워지지 않았다.
        return if rl.is_object() && rl.as_object().map(|o| o.is_empty()).unwrap_or(true) {
            Usage::empty("waiting")
        } else {
            Usage::empty("shape_changed")
        };
    }
    Usage {
        source: "statusline",
        status: if age > STALE_AFTER_SECS { "stale" } else { "ok" },
        fetched_at,
        five_hour: five,
        seven_day: window(rl, "seven_day"),
        // statusline JSON에는 모델별 창이 없다 (공식 문서: five_hour / seven_day / spend_limit 뿐).
        model_window: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 임시 폴더를 Claude 설정 폴더인 것처럼 꾸며 read()를 통째로 돌린다.
    /// statusline 훅의 실제 실행은 대화형 Claude Code 세션이 있어야 하므로, 여기서는
    /// 훅이 남기는 파일 형식만 재현해 정규화 규칙을 검증한다.
    fn stage(status: Option<&str>, installed: bool) -> PathBuf {
        let dir = env::temp_dir().join(format!("cmu-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("usage-monitor")).unwrap();
        let cmd = if installed { HOOK_CMD } else { "echo hi" };
        fs::write(dir.join("settings.json"), json!({ "statusLine": { "command": cmd } }).to_string()).unwrap();
        if let Some(s) = status {
            fs::write(dir.join("usage-monitor/status.json"), s).unwrap();
        }
        env::set_var("CLAUDE_CONFIG_DIR", &dir);
        dir
    }

    #[test]
    fn normalizes_statusline_json() {
        // 훅 미설치
        stage(None, false);
        assert_eq!(read().status, "no_source");

        // 설치했으나 아직 값 없음
        stage(None, true);
        assert_eq!(read().status, "waiting");

        // Pro 계정: 5시간 창만 있고 Opus 창은 없다 (창 부재 허용 — ARCHITECTURE §4)
        stage(Some(r#"{"rate_limits":{"five_hour":{"used_percentage":42,"resets_at":4102444800}}}"#), true);
        let u = read();
        assert_eq!(u.status, "ok");
        assert_eq!(u.source, "statusline");
        assert_eq!(u.five_hour.as_ref().unwrap().used_pct, 42.0);
        assert!(u.five_hour.as_ref().unwrap().resets_at.as_ref().unwrap().starts_with("2100-01-01"));
        assert!(u.seven_day.is_none() && u.model_window.is_none());

        // Max 계정: 세 창 모두
        stage(Some(r#"{"rate_limits":{"five_hour":{"used_percentage":74,"resets_at":4102444800},
             "seven_day":{"used_percentage":47,"resets_at":4102444800},
             "seven_day_opus":{"used_percentage":12,"resets_at":4102444800}}}"#), true);
        let u = read();
        assert_eq!(u.seven_day.unwrap().used_pct, 47.0);
        // 모델별 창은 statusline에 없다 — 옵트인 경로(oauth.rs)의 테스트가 담당한다.
        assert!(u.model_window.is_none());

        // rate_limits 자체가 없다 = Pro/Max가 아니거나 세션의 첫 응답 전
        stage(Some(r#"{"session_id":"x","model":{"id":"claude-opus-5"}}"#), true);
        assert_eq!(read().status, "waiting");

        // 형식이 바뀌었다 (아는 창이 하나도 없다)
        stage(Some(r#"{"rate_limits":{"weekly_quota":{"pct":10}}}"#), true);
        assert_eq!(read().status, "shape_changed");

        // JSON이 아니다
        stage(Some("not json"), true);
        assert_eq!(read().status, "shape_changed");

        env::remove_var("CLAUDE_CONFIG_DIR");
    }

    #[test]
    fn wraps_and_restores_existing_statusline() {
        let dir = env::temp_dir().join(format!("cmu-wrap-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // 사용자가 이미 쓰던 statusline과, 건드리면 안 되는 다른 설정.
        fs::write(dir.join("settings.json"),
            json!({ "model": "opus", "statusLine": { "type": "command", "command": "my-own-statusline.sh" } }).to_string()).unwrap();
        env::set_var("CLAUDE_CONFIG_DIR", &dir);

        install().unwrap();
        assert!(is_installed());
        let after: Value = serde_json::from_str(&fs::read_to_string(dir.join("settings.json")).unwrap()).unwrap();
        assert_eq!(after["model"], "opus", "다른 설정을 건드리면 안 된다");
        assert_eq!(fs::read_to_string(dir.join("usage-monitor/next-command")).unwrap(), "my-own-statusline.sh");
        assert!(dir.join("settings.json.usage-monitor-backup").exists(), "백업을 남겨야 한다");

        // 두 번 설치해도 원래 명령을 잃지 않는다.
        install().unwrap();
        assert_eq!(fs::read_to_string(dir.join("usage-monitor/next-command")).unwrap(), "my-own-statusline.sh");

        uninstall().unwrap();
        assert!(!is_installed());
        let back: Value = serde_json::from_str(&fs::read_to_string(dir.join("settings.json")).unwrap()).unwrap();
        assert_eq!(back["statusLine"]["command"], "my-own-statusline.sh", "원래 명령을 되돌려야 한다");
        env::remove_var("CLAUDE_CONFIG_DIR");
    }
}

// ---------- 감시 스레드 ----------

/// 옵트인 조회 주기 — 토큰 단위로 제한이 걸리므로 180초 밑으로 내리지 않는다 (조사 §B.1.1).
const OAUTH_EVERY: Duration = Duration::from_secs(180);

pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    thread::spawn(move || {
        let mut last = String::new();
        let mut oauth_at: Option<std::time::Instant> = None;
        let mut oauth_val: Option<Usage> = None;
        loop {
            let want_oauth = app.state::<crate::settings::Store>().0.lock().map(|s| s.oauth).unwrap_or(false);
            if want_oauth {
                let due = oauth_at.map(|t| t.elapsed() >= OAUTH_EVERY).unwrap_or(true);
                if due {
                    oauth_at = Some(std::time::Instant::now());
                    oauth_val = claude_dir().map(|d| crate::oauth::fetch(&d));
                }
            } else {
                oauth_val = None;
                oauth_at = None;
            }
            // 옵트인 조회가 성공했으면 그 값을 쓴다 — 모델별 창은 이 경로에만 있다.
            let u = match &oauth_val {
                Some(o) if o.status == "ok" => o.clone(),
                Some(o) => { let s = read(); if s.status == "ok" { s } else { o.clone() } }
                None => read(),
            };
            // 값이 바뀌었을 때만 보낸다 (창은 항상 마지막 값을 들고 있다).
            let key = serde_json::to_string(&u).unwrap_or_default();
            if key != last {
                last = key;
                let _ = app.emit("usage:update", &u);
            }
            thread::sleep(POLL);
        }
    });
}
