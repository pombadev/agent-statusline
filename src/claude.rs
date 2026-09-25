use chrono::{Local, TimeZone, Utc};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::Segments;
use crate::colors::*;
use crate::git::get_git_info;
use crate::time::{format_duration, format_epoch_time, parse_to_epoch};
use crate::utils::{get_f64, get_i64};

pub fn render(json: &Value, home: &str, segments: &Segments) {
    // ── Model & Effort ──
    let model_val = &json["model"];
    let raw_display_name = model_val["display_name"]
        .as_str()
        .or_else(|| model_val["displayName"].as_str())
        .or_else(|| model_val["name"].as_str())
        .or_else(|| model_val.as_str())
        .unwrap_or("");

    let model_id = model_val["id"].as_str().unwrap_or("");

    let model_label = if !model_id.is_empty() {
        model_id
    } else if !raw_display_name.is_empty() {
        raw_display_name
    } else {
        "Claude"
    };

    let mut effort = json["effort"]["level"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_default();

    if effort.is_empty() && !home.is_empty() {
        let settings_path = format!("{}/.claude/settings.json", home);
        if let Ok(content) = fs::read_to_string(&settings_path) {
            if let Ok(settings) = serde_json::from_str::<Value>(&content) {
                if let Some(e) = settings["effortLevel"].as_str() {
                    effort = e.to_string();
                }
            }
        }
    }

    if effort.is_empty() {
        effort = "default".to_string();
    }

    // ── Context Window % ──
    let ctx = &json["context_window"];
    let mut ctx_pct = get_f64(&ctx["used_percentage"]).map(|p| p.round() as i64);

    if ctx_pct.is_none() {
        let size = get_i64(&ctx["context_window_size"]).unwrap_or(0);
        if size > 0 {
            let usage = &ctx["current_usage"];
            let input_tok = get_i64(&usage["input_tokens"]).unwrap_or(0);
            let cache_create = get_i64(&usage["cache_creation_input_tokens"]).unwrap_or(0);
            let cache_read = get_i64(&usage["cache_read_input_tokens"]).unwrap_or(0);
            let total = input_tok + cache_create + cache_read;
            ctx_pct = Some((total * 100) / size);
        }
    }

    // ── CWD & Git ──
    let mut dirname = String::new();
    let mut git_branch = String::new();
    let mut git_dirty = String::new();

    if segments.show("dir") || segments.show("git") {
        let current_dir_fallback = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let cwd_str = json["cwd"]
            .as_str()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                json["workspace"]["current_dir"]
                    .as_str()
                    .filter(|s| !s.is_empty())
            })
            .or_else(|| {
                json["workspace"]["project_dir"]
                    .as_str()
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| current_dir_fallback.to_str().unwrap_or("."));
        let cwd_path = Path::new(cwd_str);

        if segments.show("dir") {
            dirname = cwd_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or(cwd_str)
                .to_string();
        }

        if segments.show("git") {
            let vcs_branch = json["vcs"]["branch"].as_str();
            let vcs_dirty = if let Some(b) = json["vcs"]["dirty"].as_bool() {
                if b { Some("*") } else { Some("") }
            } else {
                json["vcs"]["dirty"].as_str()
            };
            (git_branch, git_dirty) = get_git_info(cwd_path, vcs_branch, vcs_dirty);
        }
    }

    // Worktree
    let mut worktree = String::new();
    if let Some(wt) = json["worktree"]["name"].as_str() {
        worktree = wt.to_string();
    }
    if worktree.is_empty() {
        if let Some(wt) = json["workspace"]["git_worktree"].as_str() {
            worktree = wt.to_string();
        }
    }

    // ── Duration / Activity ──
    let now_epoch = Utc::now().timestamp();
    let mut session_duration = String::new();

    if let Some(dur_ms) = get_i64(&json["cost"]["total_duration_ms"]) {
        session_duration = format_duration(dur_ms);
    } else if let Some(start_epoch) = parse_to_epoch(&json["session"]["start_time"]) {
        let elapsed = (now_epoch - start_epoch).max(0);
        session_duration = format_duration(elapsed * 1000);
    }

    // ── Assemble Line 1 ──
    let effort_color = match effort.as_str() {
        "high" => MAGENTA,
        "medium" => WHITE,
        _ => DIM,
    };

    let mut main_parts = Vec::new();
    if segments.show("model") {
        main_parts.push(format!(
            "{BLUE}{model_label}{RESET}{DIM}:{RESET}{effort_color}{effort}{RESET}"
        ));
    }
    if segments.show("ctx") {
        if let Some(pct) = ctx_pct {
            main_parts.push(format!(
                "{DIM}ctx:{RESET}{}{pct}%{RESET}",
                color_for_pct(pct)
            ));
        }
    }
    if segments.show("dir") {
        main_parts.push(format!("{DIM}dir:{RESET}{CYAN}{dirname}{RESET}"));
    }
    if segments.show("git") && !git_branch.is_empty() {
        main_parts.push(format!(
            "{DIM}git:{RESET}{GREEN}{git_branch}{RESET}{RED}{git_dirty}{RESET}"
        ));
    }
    if segments.show("wt") && !worktree.is_empty() {
        main_parts.push(format!("{YELLOW}wt:{worktree}{RESET}"));
    }
    if segments.show("act") && !session_duration.is_empty() {
        main_parts.push(format!("{DIM}act:{RESET}{WHITE}{session_duration}{RESET}"));
    }

    // Remote Session Bridge (rc)
    let bridge_session = if segments.show("rc") {
        json["remote"]["session_id"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| env::var("CLAUDE_CODE_BRIDGE_SESSION_ID").ok())
            .unwrap_or_default()
    } else {
        String::new()
    };

    if !bridge_session.is_empty() {
        main_parts.push(format!(
            "rc:\x1b]8;;https://claude.ai/code/{}\x1b\\{}\x1b]8;;\x1b\\",
            bridge_session, bridge_session
        ));
    }

    let mut line1 = main_parts.join(SEP);

    // ── Rates from .rate_limits or cache ──
    let bar_width = 10;
    let mut rate_lines = Vec::new();

    let mut five_pct =
        get_f64(&json["rate_limits"]["five_hour"]["used_percentage"]).map(|p| p.round() as i64);
    let mut five_reset = parse_to_epoch(&json["rate_limits"]["five_hour"]["resets_at"]);
    let mut seven_pct =
        get_f64(&json["rate_limits"]["seven_day"]["used_percentage"]).map(|p| p.round() as i64);
    let mut seven_reset = parse_to_epoch(&json["rate_limits"]["seven_day"]["resets_at"]);

    let mut usage_age: Option<i64> = None;
    let mut extra_enabled = false;
    let mut extra_utilization: Option<f64> = None;
    let mut extra_used_raw: Option<f64> = None;
    let mut extra_limit_raw: Option<f64> = None;

    if five_pct.is_none() && seven_pct.is_none() {
        let xdg_runtime =
            env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| format!("{}/.cache", home));
        let cache_path = format!("{}/claude/statusline-usage-cache.json", xdg_runtime);
        if let Ok(metadata) = fs::metadata(&cache_path) {
            if let Ok(mtime) = metadata.modified() {
                if let Ok(dur) = mtime.elapsed() {
                    usage_age = Some(dur.as_secs() as i64);
                }
            }
            if let Ok(cache_str) = fs::read_to_string(&cache_path) {
                if let Ok(cache_json) = serde_json::from_str::<Value>(&cache_str) {
                    five_pct =
                        get_f64(&cache_json["five_hour"]["utilization"]).map(|p| p.round() as i64);
                    five_reset = parse_to_epoch(&cache_json["five_hour"]["resets_at"]);
                    seven_pct =
                        get_f64(&cache_json["seven_day"]["utilization"]).map(|p| p.round() as i64);
                    seven_reset = parse_to_epoch(&cache_json["seven_day"]["resets_at"]);

                    extra_enabled = cache_json["extra_usage"]["is_enabled"]
                        .as_bool()
                        .unwrap_or(false);
                    extra_utilization = get_f64(&cache_json["extra_usage"]["utilization"]);
                    extra_used_raw = get_f64(&cache_json["extra_usage"]["used_credits"]);
                    extra_limit_raw = get_f64(&cache_json["extra_usage"]["monthly_limit"]);
                }
            }
        }
    }

    let stale_note = if let Some(age) = usage_age {
        if age >= 900 {
            format!(" {}~{}m old{}", DIM, age / 60, RESET)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    if let Some(pct) = five_pct.filter(|_| segments.show("current")) {
        let mut reset_str = String::new();
        if let Some(epoch) = five_reset {
            reset_str = format_epoch_time(epoch, "time");
        }
        let bar = build_bar(pct, bar_width);
        let pct_color = color_for_pct(pct);
        let pct_fmt = format!("{:>3}", pct);
        let mut line = format!(
            "{}current{} {} {}{}%{}",
            WHITE, RESET, bar, pct_color, pct_fmt, RESET
        );
        if !reset_str.is_empty() {
            line.push_str(&format!(
                " {}⟳{} {}{}{}",
                DIM, RESET, WHITE, reset_str, RESET
            ));
        }
        line.push_str(&stale_note);
        rate_lines.push(line);
    }

    if let Some(pct) = seven_pct.filter(|_| segments.show("weekly")) {
        let mut reset_str = String::new();
        if let Some(epoch) = seven_reset {
            reset_str = format_epoch_time(epoch, "datetime");
        }
        let bar = build_bar(pct, bar_width);
        let pct_color = color_for_pct(pct);
        let pct_fmt = format!("{:>3}", pct);
        let mut line = format!(
            "{}weekly{}  {} {}{}%{}",
            WHITE, RESET, bar, pct_color, pct_fmt, RESET
        );
        if !reset_str.is_empty() {
            line.push_str(&format!(
                " {}⟳{} {}{}{}",
                DIM, RESET, WHITE, reset_str, RESET
            ));
        }
        rate_lines.push(line);
    }

    if extra_enabled && segments.show("extra") {
        if let (Some(used_raw), Some(limit_raw)) = (extra_used_raw, extra_limit_raw) {
            let extra_pct = extra_utilization.map(|p| p.round() as i64).unwrap_or(0);
            let extra_used = used_raw / 100.0;
            let extra_limit = limit_raw / 100.0;
            let bar = build_bar(extra_pct, bar_width);
            let pct_color = color_for_pct(extra_pct);

            let now_dt = Local::now();
            let next_month_dt = if now_dt.format("%m").to_string() == "12" {
                "jan 1".to_string()
            } else {
                let m = now_dt.format("%m").to_string().parse::<u32>().unwrap_or(1) + 1;
                let dt_opt = Local.with_ymd_and_hms(
                    now_dt.format("%Y").to_string().parse().unwrap_or(2026),
                    m,
                    1,
                    0,
                    0,
                    0,
                );
                if let chrono::LocalResult::Single(t) = dt_opt {
                    t.format("%b %-d").to_string().to_lowercase()
                } else {
                    String::new()
                }
            };

            let line = format!(
                "{WHITE}extra{RESET}   {bar} {pct_color}${extra_used:.2}{DIM}/{RESET}{WHITE}${extra_limit:.2}{RESET} {DIM}⟳{RESET} {WHITE}{next_month_dt}{RESET}"
            );
            rate_lines.push(line);
        }
    }

    if segments.show("tag") {
        let term_width = crate::utils::get_terminal_width(json);
        if let Some(last_rate) = rate_lines.last_mut() {
            crate::utils::attach_bottom_right_tag(last_rate, term_width);
        } else {
            crate::utils::attach_bottom_right_tag(&mut line1, term_width);
        }
    }

    if !rate_lines.is_empty() {
        if !line1.is_empty() {
            line1.push_str("\n\n");
        }
        line1.push_str(&rate_lines.join("\n"));
    }
    print!("{}", line1);
}
