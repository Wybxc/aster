use std::io::Write;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use aster::BuildOutcome;
use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};
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

pub fn emit_warning(message: &str) {
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

pub fn emit_serving(address: SocketAddr) {
    let mut writer = writer();
    let _ = writer.set_color(
        ColorSpec::new()
            .set_fg(Some(termcolor::Color::Cyan))
            .set_bold(true),
    );
    let _ = write!(writer, "server");
    let _ = writer.reset();
    let _ = writeln!(writer, "  http://{address}/");
}

pub fn emit_initialized(project: &Path) {
    let mut writer = writer();
    let _ = writer.set_color(
        ColorSpec::new()
            .set_fg(Some(termcolor::Color::Green))
            .set_bold(true),
    );
    let _ = write!(writer, "init");
    let _ = writer.reset();
    let _ = writeln!(writer, "  {}", project.display());
}

pub fn report_build(outcome: &BuildOutcome) {
    for warning in &outcome.warnings {
        emit_warning(warning.as_str());
    }
    emit_summary(outcome.outputs.len(), outcome.elapsed);
}
