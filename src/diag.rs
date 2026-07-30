use std::io::Write;
use std::path::Path;
use std::time::Duration;

use termcolor::{ColorChoice, ColorSpec, NoColor, StandardStream, WriteColor};
use typst::diag::SourceDiagnostic;
use typst_kit::diagnostics::{self, DiagnosticFormat, DiagnosticWorld};

pub fn format_diags(world: &impl DiagnosticWorld, diags: &[SourceDiagnostic]) -> String {
    let mut buffer = Vec::new();
    {
        let mut writer = NoColor::new(&mut buffer);
        if diagnostics::emit(&mut writer, world, diags.iter(), DiagnosticFormat::Human).is_err() {
            for diagnostic in diags {
                let _ = writeln!(writer, "error: {diagnostic:?}");
            }
        }
    }
    String::from_utf8_lossy(&buffer).trim_end().to_owned()
}

pub fn format_warning(world: &impl DiagnosticWorld, warning: &SourceDiagnostic) -> String {
    let formatted = format_diags(world, std::slice::from_ref(warning));
    formatted
        .strip_prefix("warning: ")
        .unwrap_or(&formatted)
        .to_owned()
}
fn writer() -> StandardStream {
    StandardStream::stderr(ColorChoice::Auto)
}

pub fn emit_summary(count: usize, elapsed: Duration) {
    let mut writer = writer();
    let _ = writer.set_color(ColorSpec::new().set_bold(true));
    let _ = write!(writer, "built {count} page");
    if count != 1 {
        let _ = write!(writer, "s");
    }
    let _ = writer.reset();
    let _ = writeln!(writer, " in {:.1}s", elapsed.as_secs_f64());
}

fn styled_warning(message: &str) {
    let mut writer = writer();
    let _ = writer.set_color(
        ColorSpec::new()
            .set_fg(Some(termcolor::Color::Yellow))
            .set_bold(true),
    );
    let _ = write!(writer, "warning");
    let _ = writer.reset();
    let _ = writeln!(writer, ": {message}");
}

#[cfg(not(test))]
pub fn emit_built_page(path: &str) {
    let mut writer = writer();
    let _ = writer.set_color(
        ColorSpec::new()
            .set_fg(Some(termcolor::Color::Green))
            .set_bold(true),
    );
    let _ = write!(writer, "build");
    let _ = writer.reset();
    let _ = writeln!(writer, "  {path}");
}

pub fn emit_warning(message: &str) {
    styled_warning(message);
}

pub fn emit_error(message: &str) {
    let mut writer = writer();
    let _ = writer.set_color(
        ColorSpec::new()
            .set_fg(Some(termcolor::Color::Red))
            .set_bold(true),
    );
    let _ = write!(writer, "error");
    let _ = writer.reset();
    let _ = writeln!(writer, ": {message}");
}

pub fn emit_watching(project: &Path) {
    let mut writer = writer();
    let _ = writer.set_color(
        ColorSpec::new()
            .set_fg(Some(termcolor::Color::Cyan))
            .set_bold(true),
    );
    let _ = write!(writer, "watch");
    let _ = writer.reset();
    let _ = writeln!(writer, "  {}", project.display());
}

pub fn emit_rebuilding() {
    let mut writer = writer();
    let _ = writer.set_color(
        ColorSpec::new()
            .set_fg(Some(termcolor::Color::Cyan))
            .set_bold(true),
    );
    let _ = write!(writer, "rebuild");
    let _ = writer.reset();
    let _ = writeln!(writer, "  change detected");
}
