//! High-level UI components: error panels, welcome panels, styled tables,
//! key-value display, and a thinking indicator.

use crate::theme;
use std::io::Write;

// ── Boxed error panel ───────────────────────────────────────────────

pub fn print_boxed_error(title: &str, message: &str, hint: Option<&str>) {
    let w = 64;
    eprintln!();
    eprintln!("  {}", theme::error_box_top(title, w));
    for line in message.lines() {
        eprintln!("  {}", theme::error_box_line(&format!("  {}", line), w,));
    }
    if let Some(hint) = hint {
        eprintln!("  {}", theme::error_box_mid(w));
        eprintln!(
            "  {}",
            theme::error_box_line(&format!("  {} {}", theme::dimmed("Hint:"), hint), w,)
        );
    }
    eprintln!("  {}", theme::error_box_bottom(w));
    eprintln!();
}

// ── Welcome panel (purple border) ───────────────────────────────────

pub fn print_welcome_panel(lines: &[String]) {
    let max_line = lines.iter().map(|l| strip_ansi_len(l)).max().unwrap_or(30);
    let w = (max_line + 6).max(40);
    println!("  {}", theme::box_top("", w));
    for line in lines {
        println!("  {}", theme::box_line(&format!("  {}", line), w));
    }
    println!("  {}", theme::box_bottom(w));
}

// ── Key-value display ───────────────────────────────────────────────

pub fn kv(key: &str, val: &str) {
    println!("  {}: {}", theme::label(key), val);
}

pub fn kv_value(key: &str, val: &str) {
    println!("  {}: {}", theme::label(key), theme::value(val));
}

pub fn kv_success(key: &str, val: &str) {
    println!("  {}: {}", theme::label(key), theme::success(val));
}

// ── Thinking indicator ──────────────────────────────────────────────

pub struct ThinkingIndicator {
    active: bool,
}

impl ThinkingIndicator {
    pub fn start(msg: &str) -> Self {
        eprint!("\r  {} {}", theme::dimmed("●"), theme::dimmed(msg));
        let _ = std::io::stderr().flush();
        Self { active: true }
    }

    pub fn done(&mut self) {
        if self.active {
            eprint!("\r{}\r", " ".repeat(60));
            let _ = std::io::stderr().flush();
            self.active = false;
        }
    }
}

impl Drop for ThinkingIndicator {
    fn drop(&mut self) {
        self.done();
    }
}

// ── Styled comfy-table builder ──────────────────────────────────────

pub fn styled_table() -> comfy_table::Table {
    let mut table = comfy_table::Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic);
    table
}

// ── Section header with gradient ────────────────────────────────────

pub fn section(title: &str) {
    println!();
    println!("  {}", theme::header(title));
    println!("  {}", theme::gradient_divider(title.len() + 4));
}

// ── Internal helpers ────────────────────────────────────────────────

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
