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
// Build output
// ---------------------------------------------------------------------------

fn writer() -> StandardStream {
    StandardStream::stderr(ColorChoice::Auto)
}

fn dimmed(w: &mut StandardStream) {
    let _ = w.set_color(ColorSpec::new().set_dimmed(true));
}

fn reset(w: &mut StandardStream) {
    let _ = w.reset();
}

/// Print a dimmed bullet for each output file.
pub fn emit_page(path: &str) {
    let mut w = writer();
    dimmed(&mut w);
    let _ = write!(w, "   {path}");
    reset(&mut w);
    let _ = writeln!(w);
}

/// Print the build summary line.
pub fn emit_summary(count: usize, elapsed: &Instant) {
    let secs = elapsed.elapsed().as_secs_f64();
    let mut w = writer();
    let _ = w.set_color(ColorSpec::new().set_bold(true));
    let _ = write!(w, "built {count} page");
    if count != 1 {
        let _ = write!(w, "s");
    }
    let _ = w.reset();
    let _ = w.set_color(ColorSpec::new().set_dimmed(true));
    let _ = writeln!(w, " in {secs:.1}s");
    let _ = w.reset();
}

// ---------------------------------------------------------------------------
// Styled diagnostics
// ---------------------------------------------------------------------------

fn styled_prefix(prefix: &str, color: Color, message: &str) {
    let mut w = writer();
    let _ = w.set_color(ColorSpec::new().set_fg(Some(color)).set_bold(true));
    let _ = write!(w, "{prefix}");
    let _ = w.reset();
    let _ = writeln!(w, ": {message}");
}

/// Print a styled `error:` message to stderr.
pub fn emit_error(message: &str) {
    styled_prefix("error", Color::Red, message);
}

/// Print a styled `warning:` message to stderr.
pub fn emit_warning(message: &str) {
    styled_prefix("warning", Color::Yellow, message);
}
