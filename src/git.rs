use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const DIRTY_CACHE_SECS: u64 = 5;

pub fn find_git_dir(start: &Path) -> Option<PathBuf> {
    let mut curr = start.to_path_buf();
    loop {
        let git = curr.join(".git");
        if git.exists() {
            return Some(git);
        }
        if !curr.pop() {
            break;
        }
    }
    None
}

pub fn get_branch_from_git_dir(git_path: &Path) -> Option<String> {
    let head_path = if git_path.is_file() {
        let content = fs::read_to_string(git_path).ok()?;
        let trimmed = content.trim();
        if let Some(rest) = trimmed.strip_prefix("gitdir:") {
            let p = rest.trim();
            let target = if Path::new(p).is_absolute() {
                PathBuf::from(p)
            } else {
                git_path.parent()?.join(p)
            };
            target.join("HEAD")
        } else {
            return None;
        }
    } else {
        git_path.join("HEAD")
    };

    let head_content = fs::read_to_string(head_path).ok()?;
    let trimmed = head_content.trim();
    if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
        Some(branch.to_string())
    } else if !trimmed.is_empty() {
        let sha: String = trimmed.chars().take(7).collect();
        Some(format!("@{}", sha))
    } else {
        None
    }
}

fn dirty_cache_path(git_path: &Path) -> Option<PathBuf> {
    let base = match env::var("XDG_RUNTIME_DIR") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(env::var("HOME").ok()?).join(".cache"),
    };
    // FNV-1a so cache names stay stable across runs
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in git_path.to_string_lossy().as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Some(
        base.join("agent-statusline")
            .join(format!("git-{:x}.dirty", hash)),
    )
}

fn dirty_count(git_path: &Path, cwd_path: &Path) -> Option<u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs());
    let cache_path = dirty_cache_path(git_path);

    if let (Some(now), Some(cache_path)) = (now, cache_path.as_deref()) {
        if let Ok(content) = fs::read_to_string(cache_path) {
            let mut parts = content.split_whitespace();
            let ts = parts.next().and_then(|s| s.parse::<u64>().ok());
            let count = parts.next().and_then(|s| s.parse::<u64>().ok());
            if let (Some(ts), Some(count)) = (ts, count) {
                if now.saturating_sub(ts) <= DIRTY_CACHE_SECS {
                    return Some(count);
                }
            }
        }
    }

    let output = Command::new("git")
        .args([
            "-C",
            cwd_path.to_str().unwrap_or("."),
            "--no-optional-locks",
            "status",
            "--porcelain",
            "-uno",
        ])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.lines().filter(|l| !l.trim().is_empty()).count() as u64;

    if let (Some(now), Some(cache_path)) = (now, cache_path) {
        if let Some(dir) = cache_path.parent() {
            if fs::create_dir_all(dir).is_ok() {
                let _ = fs::write(cache_path, format!("{} {}", now, count));
            }
        }
    }
    Some(count)
}

pub fn get_git_info(
    cwd_path: &Path,
    vcs_branch: Option<&str>,
    vcs_dirty: Option<&str>,
) -> (String, String) {
    let branch_max = 20;

    if let Some(branch) = vcs_branch {
        if !branch.is_empty() {
            let mut b = branch.to_string();
            if b.chars().count() > branch_max {
                let truncated: String = b.chars().take(branch_max - 1).collect();
                b = format!("{}…", truncated);
            }
            let d = vcs_dirty.unwrap_or("").to_string();
            return (b, d);
        }
    }

    let git_path_opt = find_git_dir(cwd_path);
    if git_path_opt.is_none() {
        return (String::new(), String::new());
    }
    let git_path = git_path_opt.unwrap();

    let mut git_branch = get_branch_from_git_dir(&git_path).unwrap_or_default();

    if git_branch.is_empty() {
        if let Ok(output) = Command::new("git")
            .args([
                "-C",
                cwd_path.to_str().unwrap_or("."),
                "symbolic-ref",
                "--short",
                "HEAD",
            ])
            .stderr(Stdio::null())
            .output()
        {
            if output.status.success() {
                git_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }
    }

    if git_branch.is_empty() {
        if let Ok(output) = Command::new("git")
            .args([
                "-C",
                cwd_path.to_str().unwrap_or("."),
                "rev-parse",
                "--short",
                "HEAD",
            ])
            .stderr(Stdio::null())
            .output()
        {
            if output.status.success() {
                let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !sha.is_empty() {
                    git_branch = format!("@{}", sha);
                }
            }
        }
    }

    if git_branch.chars().count() > branch_max {
        let truncated: String = git_branch.chars().take(branch_max - 1).collect();
        git_branch = format!("{}…", truncated);
    }

    let mut git_dirty = String::new();
    if !git_branch.is_empty() {
        if let Some(dirty) = vcs_dirty {
            git_dirty = dirty.to_string();
        } else if let Some(count) = dirty_count(&git_path, cwd_path) {
            if count > 0 {
                git_dirty = format!("*{}", count);
            }
        }
    }

    (git_branch, git_dirty)
}
