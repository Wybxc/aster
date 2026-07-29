use std::io::Write;

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

fn styled_prefix(prefix: &str, color: Color, message: &str) {
    let mut w = StandardStream::stderr(ColorChoice::Auto);
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
