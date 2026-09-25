mod antigravity;
mod claude;
mod colors;
mod git;
mod time;
mod utils;

use serde_json::Value;
use std::env;
use std::io::{self, Read};

#[derive(Copy, Clone, PartialEq, Eq)]
enum Mode {
    Claude,
    Antigravity,
}

#[derive(Default)]
struct Segments {
    hidden: u16,
}

impl Segments {
    fn bit(name: &str) -> Option<u16> {
        Some(match name {
            "model" => 1 << 0,
            "ctx" => 1 << 1,
            "dir" => 1 << 2,
            "git" => 1 << 3,
            "wt" => 1 << 4,
            "act" => 1 << 5,
            "rc" => 1 << 6,
            "current" => 1 << 7,
            "weekly" => 1 << 8,
            "extra" => 1 << 9,
            "current-3p" => 1 << 10,
            "weekly-3p" => 1 << 11,
            "tag" => 1 << 12,
            _ => return None,
        })
    }

    fn set(&mut self, name: &str, show: bool) -> bool {
        let Some(bit) = Self::bit(name) else {
            return false;
        };
        if show {
            self.hidden &= !bit;
        } else {
            self.hidden |= bit;
        }
        true
    }

    fn show(&self, name: &str) -> bool {
        Self::bit(name).is_some_and(|bit| self.hidden & bit == 0)
    }
}

fn print_help() {
    println!(
        "agent-statusline {}
Fast statusline generator for Antigravity CLI and Claude Code

USAGE:
    agent-statusline [OPTIONS] < JSON_INPUT

OPTIONS:
    -a, --agy, --antigravity    Force Antigravity mode
    -c, --claude                Force Claude Code mode
        --no-SEGMENT            Hide one segment (see names below)
        --show-SEGMENT          Show one segment (enabled by default)
    -h, --help                  Print help information
    -V, -v, --version           Print version information

SEGMENTS:
    model, ctx, dir, git, wt, act, rc,
    current, weekly, extra, current-3p, weekly-3p, tag",
        env!("CARGO_PKG_VERSION")
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut forced_mode: Option<Mode> = None;
    let mut segments = Segments::default();

    for arg in &args[1..] {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--version" | "-V" | "-v" => {
                println!("agent-statusline {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--claude" | "-c" => forced_mode = Some(Mode::Claude),
            "--agy" | "--antigravity" | "-a" => forced_mode = Some(Mode::Antigravity),
            _ => {
                if let Some(name) = arg.strip_prefix("--no-") {
                    segments.set(name, false);
                } else if let Some(name) = arg.strip_prefix("--show-") {
                    segments.set(name, true);
                }
            }
        }
    }

    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() || input.trim_ascii().is_empty() {
        let mode = forced_mode.unwrap_or_else(|| {
            if args.get(0).map(|s| s.contains("claude")).unwrap_or(false) {
                Mode::Claude
            } else {
                Mode::Antigravity
            }
        });
        let mut name = (if mode == Mode::Claude {
            "Claude"
        } else {
            "Gemini"
        })
        .to_string();
        if !segments.show("model") {
            name.clear();
        }
        if segments.show("tag") {
            let term_width = utils::get_terminal_width(&Value::Null);
            utils::attach_bottom_right_tag(&mut name, term_width);
        }
        print!("{}", name);
        return;
    }

    let json: Value = match serde_json::from_slice(&input) {
        Ok(v) => v,
        Err(_) => {
            let mode = forced_mode.unwrap_or(Mode::Antigravity);
            let mut name = (if mode == Mode::Claude {
                "Claude"
            } else {
                "Gemini"
            })
            .to_string();
            if !segments.show("model") {
                name.clear();
            }
            if segments.show("tag") {
                let term_width = utils::get_terminal_width(&Value::Null);
                utils::attach_bottom_right_tag(&mut name, term_width);
            }
            print!("{}", name);
            return;
        }
    };

    let mode = forced_mode.unwrap_or_else(|| {
        if json["product"].as_str() == Some("antigravity") || json["quota"].is_object() {
            Mode::Antigravity
        } else if json["rate_limits"].is_object()
            || json["worktree"].is_object()
            || json["remote"].is_object()
            || json["product"].as_str() == Some("claude")
        {
            Mode::Claude
        } else if args.get(0).map(|s| s.contains("claude")).unwrap_or(false) {
            Mode::Claude
        } else {
            Mode::Antigravity
        }
    });

    let home = env::var("HOME").unwrap_or_default();

    match mode {
        Mode::Claude => claude::render(&json, &home, &segments),
        Mode::Antigravity => antigravity::render(&json, &home, &segments),
    }
}
