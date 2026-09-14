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

fn print_help() {
    println!(
        "agent-statusline {}
Fast statusline generator for Antigravity CLI and Claude Code

USAGE:
    agent-statusline [OPTIONS] < JSON_INPUT

OPTIONS:
    -a, --agy, --antigravity    Force Antigravity mode
    -c, --claude                Force Claude Code mode
        --show-tag              Show generator tag (enabled by default)
        --no-tag                Do not show generator tag
    -h, --help                  Print help information
    -V, -v, --version           Print version information",
        env!("CARGO_PKG_VERSION")
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut forced_mode: Option<Mode> = None;
    let mut show_tag = true;

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
            "--show-tag" | "--show-tag=true" => show_tag = true,
            "--no-tag" | "--no-show-tag" | "--show-tag=false" => show_tag = false,
            _ => {}
        }
    }

    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() || input.trim().is_empty() {
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
        if show_tag {
            let term_width = utils::get_terminal_width(&Value::Null);
            utils::attach_bottom_right_tag(&mut name, term_width);
        }
        print!("{}", name);
        return;
    }

    let json: Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => {
            let mode = forced_mode.unwrap_or(Mode::Antigravity);
            let mut name = (if mode == Mode::Claude {
                "Claude"
            } else {
                "Gemini"
            })
            .to_string();
            if show_tag {
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
        Mode::Claude => claude::render(&json, &home, show_tag),
        Mode::Antigravity => antigravity::render(&json, &home, show_tag),
    }
}
