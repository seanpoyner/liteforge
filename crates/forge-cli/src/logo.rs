//! ASCII art logos for the Forge CLI.

use crate::theme;

/// Large logo for `--help` and `serve` startup.
pub fn print_large() {
    println!();
    println!("  {}", theme::gradient_divider(68));
    println!();

    let forge = [
        r"  ███████╗ ██████╗ ██████╗  ██████╗ ███████╗",
        r"  ██╔════╝██╔═══██╗██╔══██╗██╔════╝ ██╔════╝",
        r"  █████╗  ██║   ██║██████╔╝██║  ███╗█████╗  ",
        r"  ██╔══╝  ██║   ██║██╔══██╗██║   ██║██╔══╝  ",
        r"  ██║     ╚██████╔╝██║  ██║╚██████╔╝███████╗",
        r"  ╚═╝      ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝",
    ];

    for line in &forge {
        println!("  {}", theme::header(line));
    }

    println!();

    let sep_left = "═══════════════════════════════════════════════════════";
    println!(
        "  {} {} {}",
        theme::pink(&sep_left[..28]),
        theme::pink("◆"),
        theme::pink(&sep_left[..28]),
    );

    println!();
    println!(
        "  {}  {}  {}",
        theme::dimmed("━━━━━━━━━━━━"),
        theme::dimmed("LiteForge — OpenAI-compatible LLM toolkit"),
        theme::dimmed("━━━━━━━━━━━━"),
    );
    println!();
    println!("  {}", theme::gradient_divider(68));
    println!();
}

/// Compact logo for interactive commands (chat, agents run, claude, infra).
pub fn print_compact() {
    println!();
    println!(
        "  {}  {}",
        theme::header("╔══╗"),
        theme::header("Forge"),
    );
    println!(
        "  {}  {}",
        theme::header("║▓▓║"),
        theme::dimmed("LiteForge CLI"),
    );
    println!("  {}", theme::header("╚══╝"),);
    println!();
}

/// Returns the large logo as a static string for clap `before_help`.
/// Embeds Dracula truecolor ANSI codes: Gold FORGE wordmark, Pink dividers.
pub fn help_banner() -> &'static str {
    concat!(
    "\n",
    "  \x1b[38;2;189;147;249m░▒▓█▓▒░\x1b[38;2;200;140;230m░▒▓█▓▒░\x1b[38;2;215;132;215m░▒▓█▓▒░\x1b[38;2;235;127;207m░▒▓█▓▒░\x1b[38;2;255;121;198m░▒▓█▓▒░\x1b[38;2;235;127;207m░▒▓█▓▒░\x1b[38;2;215;132;215m░▒▓█▓▒░\x1b[38;2;200;140;230m░▒▓█▓▒░\x1b[38;2;189;147;249m░▒▓█▓▒░\x1b[38;2;200;140;230m░▒▓█▓▒░\x1b[0m\n",
    "\n",
    "  \x1b[1;38;2;212;168;67m███████╗ ██████╗ ██████╗  ██████╗ ███████╗\x1b[0m\n",
    "  \x1b[1;38;2;212;168;67m██╔════╝██╔═══██╗██╔══██╗██╔════╝ ██╔════╝\x1b[0m\n",
    "  \x1b[1;38;2;212;168;67m█████╗  ██║   ██║██████╔╝██║  ███╗█████╗\x1b[0m\n",
    "  \x1b[1;38;2;212;168;67m██╔══╝  ██║   ██║██╔══██╗██║   ██║██╔══╝\x1b[0m\n",
    "  \x1b[1;38;2;212;168;67m██║     ╚██████╔╝██║  ██║╚██████╔╝███████╗\x1b[0m\n",
    "  \x1b[1;38;2;212;168;67m╚═╝      ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝\x1b[0m\n",
    "\n",
    "  \x1b[38;2;255;121;198m════════════════════════════ ◆ ════════════════════════════\x1b[0m\n",
    "\n",
    "  \x1b[38;2;98;114;164m━━━━━━━━━━  LiteForge — OpenAI-compatible LLM toolkit  ━━━━━━━━━━\x1b[0m\n",
    "\n",
    "  \x1b[38;2;189;147;249m░▒▓█▓▒░\x1b[38;2;200;140;230m░▒▓█▓▒░\x1b[38;2;215;132;215m░▒▓█▓▒░\x1b[38;2;235;127;207m░▒▓█▓▒░\x1b[38;2;255;121;198m░▒▓█▓▒░\x1b[38;2;235;127;207m░▒▓█▓▒░\x1b[38;2;215;132;215m░▒▓█▓▒░\x1b[38;2;200;140;230m░▒▓█▓▒░\x1b[38;2;189;147;249m░▒▓█▓▒░\x1b[38;2;200;140;230m░▒▓█▓▒░\x1b[0m\n",
    )
}
