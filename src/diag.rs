use std::io::Write;
use std::time::Instant;

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use typst::diag::SourceDiagnostic;
use typst_kit::diagnostics::{self, DiagnosticFormat, DiagnosticWorld};

/// Print Typst diagnostics to stderr using the given [`DiagnosticWorld`].
pub fn emit_diags(world: &impl DiagnosticWorld, diags: &[SourceDiagnostic]) {
    let mut writer = StandardStream::stderr(ColorChoice::Auto);
    if diagnostics::emit(&mut writer, world, diags.iter(), DiagnosticFormat::Human).is_err() {
        for diag in diags {
            eprintln!("error: {diag:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Style A: No prefix — for ambient progress and conclusive summaries.
// ---------------------------------------------------------------------------

fn writer() -> StandardStream {
    StandardStream::stderr(ColorChoice::Auto)
}

/// Print a phase message (ambient progress, no prefix).
pub fn emit_step(message: &str) {
    let mut w = writer();
    let _ = writeln!(w, "{message}");
}

/// Print the build summary (conclusive result, bold emphasis).
pub fn emit_summary(count: usize, elapsed: &Instant) {
    let secs = elapsed.elapsed().as_secs_f64();
    let mut w = writer();
    let _ = w.set_color(ColorSpec::new().set_bold(true));
    let _ = write!(w, "built {count} page");
    if count != 1 {
        let _ = write!(w, "s");
    }
    let _ = w.reset();
    let _ = writeln!(w, " in {secs:.1}s");
}

// ---------------------------------------------------------------------------
// Style B: With prefix — for concrete, actionable messages.
// ---------------------------------------------------------------------------

fn styled_prefix(prefix: &str, color: Color, message: &str) {
    let mut w = writer();
    let _ = w.set_color(ColorSpec::new().set_fg(Some(color)).set_bold(true));
    let _ = write!(w, "{prefix}");
    let _ = w.reset();
    let _ = writeln!(w, ": {message}");
}

/// Print an output file line (action: green `write` prefix).
pub fn emit_page(path: &str) {
    let mut w = writer();
    let _ = w.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true));
    let _ = write!(w, "write");
    let _ = w.reset();
    let _ = writeln!(w, "  {path}");
}

/// Print a styled `error:` message to stderr.
pub fn emit_error(message: &str) {
    styled_prefix("error", Color::Red, message);
}

/// Print a styled `warning:` message to stderr.
pub fn emit_warning(message: &str) {
    styled_prefix("warning", Color::Yellow, message);
}
