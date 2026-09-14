use chrono::Utc;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::colors::*;
use crate::git::get_git_info;
use crate::time::{format_duration, format_epoch_time, parse_to_epoch};
use crate::utils::{get_f64, get_i64};

fn strip_effort_suffix(label: &str) -> String {
    let lower = label.to_lowercase();
    for suffix in &[" (high)", " (medium)", " (low)"] {
        if let Some(pos) = lower.rfind(suffix) {
            let mut result = label[..pos].to_string();
            result.push_str(&label[pos + suffix.len()..]);
            return result.trim_end().to_string();
        }
    }
    label.to_string()
}

fn extract_effort_from_label(label: &str) -> Option<&'static str> {
    let lower = label.to_lowercase();
    if lower.contains("(high)") {
        Some("high")
    } else if lower.contains("(medium)") {
        Some("medium")
    } else if lower.contains("(low)") {
        Some("low")
    } else {
        None
    }
}

pub fn render(json: &Value, home: &str, show_tag: bool) {
    // ── Model & Effort ──
    let model_val = &json["model"];
    let raw_display_name = model_val["display_name"]
        .as_str()
        .or_else(|| model_val["displayName"].as_str())
        .or_else(|| model_val["name"].as_str())
        .or_else(|| model_val.as_str())
        .unwrap_or("");

    let model_id = model_val["id"].as_str().unwrap_or("");

    let initial_label = if !raw_display_name.is_empty() {
        raw_display_name
    } else if !model_id.is_empty() {
        model_id
    } else {
        "Gemini"
    };

    let mut effort = json["effort"]["level"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_default();

    if effort.is_empty() && !home.is_empty() {
        let settings_path = format!("{}/.gemini/antigravity-cli/settings.json", home);
        if let Ok(content) = fs::read_to_string(&settings_path) {
            if let Ok(settings) = serde_json::from_str::<Value>(&content) {
                if let Some(e) = settings["effortLevel"].as_str() {
                    effort = e.to_string();
                }
            }
        }
    }

    if effort.is_empty() {
        if let Some(e) = extract_effort_from_label(initial_label) {
            effort = e.to_string();
        }
    }

    let model_label = strip_effort_suffix(initial_label);

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
    let dirname = cwd_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(cwd_str);

    let vcs_branch = json["vcs"]["branch"].as_str();
    let vcs_dirty = if let Some(b) = json["vcs"]["dirty"].as_bool() {
        if b { Some("*") } else { Some("") }
    } else {
        json["vcs"]["dirty"].as_str()
    };

    let (git_branch, git_dirty) = get_git_info(cwd_path, vcs_branch, vcs_dirty);
    let git_worktree = json["workspace"]["git_worktree"].as_str().unwrap_or("");

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

    let mut line1 = String::with_capacity(256);
    line1.push_str(BLUE);
    line1.push_str(&model_label);
    line1.push_str(RESET);

    if !effort.is_empty() {
        line1.push_str(DIM);
        line1.push(':');
        line1.push_str(RESET);
        line1.push_str(effort_color);
        line1.push_str(&effort);
        line1.push_str(RESET);
    }

    if let Some(pct) = ctx_pct {
        line1.push_str(SEP);
        line1.push_str(DIM);
        line1.push_str("ctx:");
        line1.push_str(RESET);
        line1.push_str(color_for_pct(pct));
        line1.push_str(&pct.to_string());
        line1.push('%');
        line1.push_str(RESET);
    }

    line1.push_str(SEP);
    line1.push_str(DIM);
    line1.push_str("dir:");
    line1.push_str(RESET);
    line1.push_str(CYAN);
    line1.push_str(dirname);
    line1.push_str(RESET);

    if !git_branch.is_empty() {
        line1.push_str(SEP);
        line1.push_str(DIM);
        line1.push_str("git:");
        line1.push_str(RESET);
        line1.push_str(GREEN);
        line1.push_str(&git_branch);
        line1.push_str(RESET);
        line1.push_str(RED);
        line1.push_str(&git_dirty);
        line1.push_str(RESET);
    }

    if !git_worktree.is_empty() {
        line1.push_str(SEP);
        line1.push_str(YELLOW);
        line1.push_str("wt:");
        line1.push_str(git_worktree);
        line1.push_str(RESET);
    }

    if !session_duration.is_empty() {
        line1.push_str(SEP);
        line1.push_str(DIM);
        line1.push_str("act:");
        line1.push_str(RESET);
        line1.push_str(WHITE);
        line1.push_str(&session_duration);
        line1.push_str(RESET);
    }

    // ── Quota & Rate Limits ──
    let bar_width = 10;
    let mut rate_lines = Vec::new();

    if let Some(quota_map) = json["quota"].as_object() {
        let model_raw_lower = raw_display_name.to_lowercase();
        let prefix = if model_raw_lower.contains("claude")
            || model_raw_lower.contains("sonnet")
            || model_raw_lower.contains("anthropic")
        {
            "claude|anthropic"
        } else if model_raw_lower.contains("gpt") || model_raw_lower.contains("openai") {
            "openai|gpt"
        } else if model_raw_lower.contains("gemini") {
            "gemini"
        } else {
            "gemini"
        };

        let matches_prefix = |k: &str| -> bool {
            let k_low = k.to_lowercase();
            for p in prefix.split('|') {
                if k_low.contains(p) {
                    return true;
                }
            }
            false
        };

        let non_3p: Vec<(&String, &Value)> = quota_map
            .iter()
            .filter(|(k, _)| !k.to_lowercase().contains("3p"))
            .collect();

        let prefix_entries: Vec<(&String, &Value)> = non_3p
            .iter()
            .cloned()
            .filter(|(k, _)| matches_prefix(k))
            .collect();

        let entries = if !prefix_entries.is_empty() {
            prefix_entries
        } else {
            non_3p
        };

        let entries_3p: Vec<(&String, &Value)> = quota_map
            .iter()
            .filter(|(k, _)| k.to_lowercase().contains("3p"))
            .collect();

        let parse_entry = |candidates: &[(&String, &Value)],
                           pattern: &[&str]|
         -> Option<(i64, Option<i64>)> {
            for (k, v) in candidates {
                let k_low = k.to_lowercase();
                if pattern.iter().any(|pat| k_low.contains(pat)) {
                    let pct_opt = get_f64(&v["used_percentage"])
                        .or_else(|| get_f64(&v["remaining_percentage"]).map(|rp| 100.0 - rp))
                        .or_else(|| get_f64(&v["remaining_fraction"]).map(|rf| (1.0 - rf) * 100.0))
                        .or_else(|| get_f64(&v["used_fraction"]).map(|uf| uf * 100.0));

                    if let Some(pct_f) = pct_opt {
                        let pct = pct_f.round() as i64;
                        let mut reset_epoch = parse_to_epoch(&v["reset_time"]);
                        if reset_epoch.is_none() {
                            if let Some(sec) = get_i64(&v["reset_in_seconds"]) {
                                reset_epoch = Some(now_epoch + sec);
                            }
                        }
                        return Some((pct, reset_epoch));
                    }
                }
            }
            None
        };

        let targets: [(&'static str, bool, &[(&String, &Value)], &[&str]); 4] = [
            (
                "current",
                true,
                &entries,
                &["current", "5h", "five", "session"],
            ),
            ("weekly", false, &entries, &["week", "7d", "seven"]),
            (
                "current (3p)",
                true,
                &entries_3p,
                &["5h", "current", "five", "session"],
            ),
            ("weekly (3p)", false, &entries_3p, &["week", "7d", "seven"]),
        ];

        for (label, is_session, cand_list, pattern) in targets {
            if let Some((pct, reset_epoch)) = parse_entry(cand_list, pattern) {
                let mut reset_str = String::new();
                if let Some(epoch) = reset_epoch {
                    if is_session && epoch > now_epoch && (epoch - now_epoch) <= 86400 {
                        reset_str = format_epoch_time(epoch, "time");
                    } else {
                        reset_str = format_epoch_time(epoch, "datetime");
                    }
                }

                let bar = build_bar(pct, bar_width);
                let pct_color = color_for_pct(pct);
                let label_fmt = format!("{:<14}", label);
                let pct_fmt = format!("{:>3}", pct);

                let mut line = format!(
                    "{}{}{}{} {} {}{}%{}",
                    WHITE, label_fmt, RESET, "", bar, pct_color, pct_fmt, RESET
                );

                if !reset_str.is_empty() {
                    line.push_str(&format!(
                        " {}⟳{} {}{}{}",
                        DIM, RESET, WHITE, reset_str, RESET
                    ));
                }

                rate_lines.push(line);
            }
        }
    }

    let term_width = crate::utils::get_terminal_width(json);

    if show_tag {
        if let Some(last_rate) = rate_lines.last_mut() {
            crate::utils::attach_bottom_right_tag(last_rate, term_width);
        } else {
            crate::utils::attach_bottom_right_tag(&mut line1, term_width);
        }
    }

    print!("{}", line1);
    if !rate_lines.is_empty() {
        print!("\n{}", rate_lines.join("\n"));
    }
}
