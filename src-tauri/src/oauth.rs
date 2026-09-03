//! 2순위 경로 — 저장된 Claude Code 토큰으로 사용량을 조회한다 (ARCHITECTURE §5.1, DECISIONS D9).
//!
//! **기본 꺼짐.** 이 경로만이 모델별 주간 창(Fable·Opus 등)을 준다 — statusline JSON에는
//! `five_hour`/`seven_day`/`spend_limit`만 있다(공식 문서 확인, 2026-09-03).
//! 토큰은 읽기만 하고 갱신하지 않으며, 메모리 밖으로 내보내지 않는다.

use crate::usage::{ModelWindow, Usage, Window};
use serde_json::Value;
use std::{fs, process::Command, time::Duration};

const URL: &str = "https://api.anthropic.com/api/oauth/usage";
/// 이 헤더가 없으면 좁은 버킷에 걸려 429가 계속 난다 (조사 §B.1.1).
const UA_FALLBACK: &str = "claude-code/2.1.259";

struct Token {
    access: String,
    expired: bool,
}

/// Claude Code가 로그인 때 저장한 토큰을 읽는다. macOS는 키체인, 그 외는 파일.
fn read_token(dir: &std::path::Path) -> Option<Token> {
    let raw = if cfg!(target_os = "macos") {
        let user = std::env::var("USER").ok()?;
        let out = Command::new("security")
            .args(["find-generic-password", "-s", "Claude Code-credentials", "-a", &user, "-w"])
            .output()
            .ok()?;
        if !out.status.success() {
            // 키체인에 없으면 파일로 떨어진 경우가 있다 (조사 §B.2).
            fs::read_to_string(dir.join(".credentials.json")).ok()?
        } else {
            String::from_utf8(out.stdout).ok()?
        }
    } else {
        fs::read_to_string(dir.join(".credentials.json")).ok()?
    };
    let v: Value = serde_json::from_str(&raw).ok()?;
    let o = v.get("claudeAiOauth")?;
    let access = o.get("accessToken")?.as_str()?.to_string();
    let expires_ms = o.get("expiresAt").and_then(Value::as_i64).unwrap_or(0);
    let now_ms = chrono::Utc::now().timestamp_millis();
    Some(Token { access, expired: expires_ms > 0 && expires_ms <= now_ms })
}

/// statusline이 남긴 파일에서 Claude Code 버전을 얻는다. 없으면 고정값.
fn user_agent(dir: &std::path::Path) -> String {
    fs::read_to_string(dir.join("usage-monitor/status.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.get("version").and_then(Value::as_str).map(|s| format!("claude-code/{s}")))
        .unwrap_or_else(|| UA_FALLBACK.to_string())
}

fn window(v: &Value, key: &str) -> Option<Window> {
    let w = v.get(key)?;
    Some(Window {
        used_pct: w.get("utilization").and_then(Value::as_f64)?,
        resets_at: w.get("resets_at").and_then(Value::as_str).map(str::to_string),
    })
}

/// `limits` 배열에서 모델별 주간 창을 꺼낸다. 여러 개면 사용률이 가장 높은 것 하나만 보여 준다.
/// 실측 형태 (2026-09-03): `{kind:"weekly_scoped", percent:14, scope:{model:{display_name:"Fable"}}}`
fn model_window(v: &Value) -> Option<ModelWindow> {
    let arr = v.get("limits")?.as_array()?;
    arr.iter()
        .filter(|l| l.get("kind").and_then(Value::as_str) == Some("weekly_scoped"))
        .filter_map(|l| {
            let label = l.get("scope")?.get("model")?.get("display_name")?.as_str()?.to_string();
            Some(ModelWindow {
                label,
                used_pct: l.get("percent").and_then(Value::as_f64)?,
                resets_at: l.get("resets_at").and_then(Value::as_str).map(str::to_string),
            })
        })
        .max_by(|a, b| a.used_pct.total_cmp(&b.used_pct))
}

/// 한 번 조회한다. 실패는 계약의 상태값으로 돌려준다 — 창은 마지막 값을 유지한다.
pub fn fetch(dir: &std::path::Path) -> Usage {
    let Some(tok) = read_token(dir) else { return Usage::empty_oauth("no_token") };
    if tok.expired {
        return Usage::empty_oauth("auth_expired");
    }
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .new_agent();
    let res = agent
        .get(URL)
        .header("Authorization", &format!("Bearer {}", tok.access))
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("User-Agent", &user_agent(dir))
        .header("Content-Type", "application/json")
        .call();

    let mut body = match res {
        Ok(r) => r,
        Err(ureq::Error::StatusCode(401)) | Err(ureq::Error::StatusCode(403)) => {
            return Usage::empty_oauth("auth_expired")
        }
        Err(ureq::Error::StatusCode(429)) => return Usage::empty_oauth("rate_limited"),
        Err(_) => return Usage::empty_oauth("unreachable"),
    };
    let Ok(text) = body.body_mut().read_to_string() else { return Usage::empty_oauth("unreachable") };
    let Ok(v) = serde_json::from_str::<Value>(&text) else { return Usage::empty_oauth("shape_changed") };

    let five = window(&v, "five_hour");
    if five.is_none() {
        return Usage::empty_oauth("shape_changed");
    }
    Usage {
        source: "oauth",
        status: "ok",
        fetched_at: Some(chrono::Utc::now().to_rfc3339()),
        five_hour: five,
        seven_day: window(&v, "seven_day"),
        model_window: model_window(&v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-09-03 실측 응답을 줄인 것. `limits`가 모델별 창의 실제 소스다.
    const SAMPLE: &str = r#"{
      "five_hour": {"utilization": 35.0, "resets_at": "2026-09-03T03:59:59+00:00"},
      "seven_day": {"utilization": 9.0, "resets_at": "2026-09-09T12:59:59+00:00"},
      "seven_day_opus": null, "seven_day_sonnet": null,
      "limits": [
        {"kind":"session","percent":35,"resets_at":"2026-09-03T03:59:59+00:00","scope":null},
        {"kind":"weekly_all","percent":9,"resets_at":"2026-09-09T12:59:59+00:00","scope":null},
        {"kind":"weekly_scoped","percent":14,"resets_at":"2026-09-09T12:59:59+00:00",
         "scope":{"model":{"id":null,"display_name":"Fable"}}}
      ]}"#;

    #[test]
    fn reads_model_window_from_limits() {
        let v: Value = serde_json::from_str(SAMPLE).unwrap();
        assert_eq!(window(&v, "five_hour").unwrap().used_pct, 35.0);
        assert_eq!(window(&v, "seven_day").unwrap().used_pct, 9.0);
        let m = model_window(&v).unwrap();
        assert_eq!(m.label, "Fable");
        assert_eq!(m.used_pct, 14.0);
        // 레거시 필드가 전부 null이어도 limits에서 읽어야 한다.
        assert!(v["seven_day_opus"].is_null());
    }

    #[test]
    fn picks_the_busiest_model_window() {
        let v: Value = serde_json::from_str(
            r#"{"limits":[
              {"kind":"weekly_scoped","percent":14,"scope":{"model":{"display_name":"Fable"}}},
              {"kind":"weekly_scoped","percent":61,"scope":{"model":{"display_name":"Opus"}}},
              {"kind":"weekly_all","percent":99,"scope":null}]}"#).unwrap();
        let m = model_window(&v).unwrap();
        assert_eq!(m.label, "Opus", "가장 많이 쓴 모델을 보여 준다");
        assert_eq!(m.used_pct, 61.0);
    }

    #[test]
    fn no_model_window_when_absent() {
        let v: Value = serde_json::from_str(r#"{"limits":[{"kind":"session","percent":3,"scope":null}]}"#).unwrap();
        assert!(model_window(&v).is_none());
    }
}
