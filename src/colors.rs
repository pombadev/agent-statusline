pub const BLUE: &str = "\x1b[38;2;0;153;255m";
pub const ORANGE: &str = "\x1b[38;2;255;176;85m";
pub const GREEN: &str = "\x1b[38;2;0;175;80m";
pub const CYAN: &str = "\x1b[38;2;86;182;194m";
pub const RED: &str = "\x1b[38;2;255;85;85m";
pub const YELLOW: &str = "\x1b[38;2;230;200;0m";
pub const WHITE: &str = "\x1b[38;2;220;220;220m";
pub const MAGENTA: &str = "\x1b[38;2;180;140;255m";
pub const DIM: &str = "\x1b[2m";
pub const RESET: &str = "\x1b[0m";
pub const SEP: &str = " \x1b[2m│\x1b[0m ";

pub fn color_for_pct(pct: i64) -> &'static str {
    if pct >= 90 {
        RED
    } else if pct >= 70 {
        YELLOW
    } else if pct >= 50 {
        ORANGE
    } else {
        GREEN
    }
}

pub fn build_bar(pct: i64, width: usize) -> String {
    let pct_clamped = pct.clamp(0, 100) as usize;
    let filled = pct_clamped * width / 100;
    let empty = width.saturating_sub(filled);
    let color = color_for_pct(pct);

    let mut bar = String::with_capacity(64);
    bar.push_str(color);
    for _ in 0..filled {
        bar.push_str("● ");
    }
    bar.push_str(DIM);
    for _ in 0..empty {
        bar.push_str("○ ");
    }
    bar.push_str(RESET);
    bar
}
