//! Dracula + brand accent theme for the Forge CLI.
//!
//! Provides truecolor constants, styled text helpers, box-drawing,
//! gradient dividers, and status icons. Falls back to 256-color or
//! basic ANSI when the terminal lacks truecolor support.

use owo_colors::OwoColorize;

// ── Dracula palette (R, G, B) ───────────────────────────────────────
pub const PURPLE: (u8, u8, u8) = (189, 147, 249);
pub const PINK: (u8, u8, u8) = (255, 121, 198);
pub const CYAN: (u8, u8, u8) = (139, 233, 253);
pub const GREEN: (u8, u8, u8) = (80, 250, 123);
pub const ORANGE: (u8, u8, u8) = (255, 184, 108);
pub const RED: (u8, u8, u8) = (255, 85, 85);
pub const YELLOW: (u8, u8, u8) = (241, 250, 140);
pub const FG: (u8, u8, u8) = (248, 248, 242);
pub const COMMENT: (u8, u8, u8) = (98, 114, 164);

// ── brand accent ───────────────────────────────────────────
pub const GOLD: (u8, u8, u8) = (212, 168, 67);

// ── Truecolor detection ─────────────────────────────────────────────

fn has_truecolor() -> bool {
    supports_color::on(supports_color::Stream::Stdout)
        .map(|c| c.has_16m)
        .unwrap_or(false)
}

fn has_color() -> bool {
    supports_color::on(supports_color::Stream::Stdout).is_some()
}

// ── Core styled-text helpers ────────────────────────────────────────

fn tc(text: &str, (r, g, b): (u8, u8, u8)) -> String {
    if has_truecolor() {
        format!("{}", text.truecolor(r, g, b))
    } else if has_color() {
        match (r, g, b) {
            c if c == PURPLE => format!("{}", text.purple()),
            c if c == PINK => format!("{}", text.magenta()),
            c if c == CYAN => format!("{}", text.cyan()),
            c if c == GREEN => format!("{}", text.green()),
            c if c == ORANGE => format!("{}", text.yellow()),
            c if c == RED => format!("{}", text.red()),
            c if c == YELLOW => format!("{}", text.yellow()),
            c if c == GOLD => format!("{}", text.yellow()),
            c if c == COMMENT => format!("{}", text.bright_black()),
            _ => text.to_string(),
        }
    } else {
        text.to_string()
    }
}

fn tc_bold(text: &str, color: (u8, u8, u8)) -> String {
    if has_truecolor() {
        let (r, g, b) = color;
        format!("{}", text.truecolor(r, g, b).bold())
    } else if has_color() {
        tc(text, color).replace(text, &text.bold().to_string())
    } else {
        text.to_string()
    }
}

// ── Public semantic helpers ─────────────────────────────────────────

pub fn header(text: &str) -> String {
    tc_bold(text, GOLD)
}
pub fn subheader(text: &str) -> String {
    tc_bold(text, PURPLE)
}
pub fn value(text: &str) -> String {
    tc(text, CYAN)
}
pub fn success(text: &str) -> String {
    tc(text, GREEN)
}
pub fn warning(text: &str) -> String {
    tc(text, ORANGE)
}
pub fn error_text(text: &str) -> String {
    tc_bold(text, RED)
}
pub fn pink(text: &str) -> String {
    tc(text, PINK)
}
pub fn dimmed(text: &str) -> String {
    tc(text, COMMENT)
}
pub fn label(text: &str) -> String {
    if has_truecolor() {
        let (r, g, b) = FG;
        format!("{}", text.truecolor(r, g, b).bold())
    } else {
        format!("{}", text.bold())
    }
}

// ── Status icons ────────────────────────────────────────────────────

pub fn check(ok: bool) -> String {
    if ok {
        success("✓")
    } else {
        error_text("✗")
    }
}

pub fn yn(ok: bool) -> String {
    if ok {
        success("Y")
    } else {
        dimmed("-")
    }
}

pub fn arrow() -> String {
    tc("→", PURPLE)
}
pub fn bullet() -> String {
    tc("●", PINK)
}
pub fn dot_leader(n: usize) -> String {
    dimmed(&"·".repeat(n))
}

// ── Dividers ────────────────────────────────────────────────────────

pub fn gradient_divider(width: usize) -> String {
    if !has_truecolor() || width == 0 {
        return dimmed(&"─".repeat(width));
    }
    let chars = ['░', '▒', '▓', '█', '▓', '▒', '░'];
    let mut out = String::with_capacity(width * 4);
    for i in 0..width {
        let t = i as f32 / width as f32;
        let ci = (t * (chars.len() - 1) as f32) as usize;
        let c = chars[ci.min(chars.len() - 1)];
        let r = lerp(PURPLE.0, PINK.0, t);
        let g = lerp(PURPLE.1, PINK.1, t);
        let b = lerp(PURPLE.2, PINK.2, t);
        out.push_str(&format!("{}", c.to_string().truecolor(r, g, b)));
    }
    out
}

pub fn thin_divider(width: usize) -> String {
    dimmed(&"─".repeat(width))
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}

// ── Box drawing ─────────────────────────────────────────────────────

pub fn box_top(title: &str, width: usize) -> String {
    let inner = width.saturating_sub(2);
    let title_display = if title.is_empty() {
        "═".repeat(inner)
    } else {
        let pad = inner.saturating_sub(title.len() + 2);
        format!(" {} {}", title, "═".repeat(pad))
    };
    tc(&format!("╔{}╗", title_display), PURPLE)
}

pub fn box_mid(width: usize) -> String {
    let inner = width.saturating_sub(2);
    tc(&format!("╠{}╣", "═".repeat(inner)), PURPLE)
}

pub fn box_bottom(width: usize) -> String {
    let inner = width.saturating_sub(2);
    tc(&format!("╚{}╝", "═".repeat(inner)), PURPLE)
}

pub fn box_line(content: &str, width: usize) -> String {
    let inner = width.saturating_sub(4);
    let visible_len = strip_ansi_len(content);
    let pad = inner.saturating_sub(visible_len);
    format!(
        "{} {}{} {}",
        tc("║", PURPLE),
        content,
        " ".repeat(pad),
        tc("║", PURPLE),
    )
}

pub fn box_empty(width: usize) -> String {
    let inner = width.saturating_sub(4);
    format!(
        "{} {} {}",
        tc("║", PURPLE),
        " ".repeat(inner),
        tc("║", PURPLE),
    )
}

// ── Error box ───────────────────────────────────────────────────────

pub fn error_box_top(title: &str, width: usize) -> String {
    let inner = width.saturating_sub(2);
    let title_display = if title.is_empty() {
        "═".repeat(inner)
    } else {
        let pad = inner.saturating_sub(title.len() + 2);
        format!(" {} {}", title, "═".repeat(pad))
    };
    tc(&format!("╔{}╗", title_display), RED)
}

pub fn error_box_mid(width: usize) -> String {
    let inner = width.saturating_sub(2);
    tc(&format!("╠{}╣", "═".repeat(inner)), RED)
}

pub fn error_box_bottom(width: usize) -> String {
    let inner = width.saturating_sub(2);
    tc(&format!("╚{}╝", "═".repeat(inner)), RED)
}

pub fn error_box_line(content: &str, width: usize) -> String {
    let inner = width.saturating_sub(4);
    let visible_len = strip_ansi_len(content);
    let pad = inner.saturating_sub(visible_len);
    format!(
        "{} {}{} {}",
        tc("║", RED),
        content,
        " ".repeat(pad),
        tc("║", RED),
    )
}

// ── Format number with commas ───────────────────────────────────────

pub fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

// ── Clap help styles ────────────────────────────────────────────────

pub fn clap_styles() -> clap::builder::Styles {
    use clap::builder::styling::{AnsiColor, Style, Styles};

    Styles::styled()
        .header(Style::new().bold().fg_color(Some(AnsiColor::Yellow.into())))
        .usage(Style::new().fg_color(Some(AnsiColor::Magenta.into())))
        .literal(Style::new().fg_color(Some(AnsiColor::Cyan.into())))
        .placeholder(Style::new().fg_color(Some(AnsiColor::Magenta.into())))
        .valid(Style::new().fg_color(Some(AnsiColor::Green.into())))
        .invalid(Style::new().fg_color(Some(AnsiColor::Red.into())))
        .error(Style::new().bold().fg_color(Some(AnsiColor::Red.into())))
}

// ── Internals ───────────────────────────────────────────────────────

fn strip_ansi_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            len += 1;
        }
    }
    len
}
