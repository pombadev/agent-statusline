#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Mode {
    Claude,
    Antigravity,
}

#[derive(Default)]
pub struct Segments {
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

    pub fn show(&self, name: &str) -> bool {
        Self::bit(name).is_some_and(|bit| self.hidden & bit == 0)
    }
}

pub enum Invocation {
    Run {
        forced_mode: Option<Mode>,
        segments: Segments,
    },
    Exit,
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

pub fn parse(argv: &[String]) -> Invocation {
    let mut forced_mode: Option<Mode> = None;
    let mut segments = Segments::default();

    for arg in &argv[1..] {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                return Invocation::Exit;
            }
            "--version" | "-V" | "-v" => {
                println!("agent-statusline {}", env!("CARGO_PKG_VERSION"));
                return Invocation::Exit;
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

    Invocation::Run {
        forced_mode,
        segments,
    }
}