mod antigravity;
mod claude;
mod cli;
mod colors;
mod git;
mod time;
mod utils;

use cli::{Invocation, Mode};
use serde_json::Value;
use std::env;
use std::io::{self, Read};

fn main() {
    let args: Vec<String> = env::args().collect();
    let Invocation::Run {
        forced_mode,
        segments,
    } = cli::parse(&args)
    else {
        return;
    };

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
