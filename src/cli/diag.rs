use std::io::Write;
use std::net::SocketAddr;
use std::path::Path;
use std::time::{Duration, Instant};

use aster::BuildOutcome;
use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};
use tracing::Subscriber;
use tracing::span::{Attributes, Id};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;

fn writer() -> StandardStream {
    StandardStream::stderr(ColorChoice::Auto)
}

pub fn init() {
    let subscriber = tracing_subscriber::registry().with(ProgressLayer);
    let _ = tracing::subscriber::set_global_default(subscriber);
}

pub fn emit_summary(page_count: usize, endpoint_count: usize, elapsed: Duration) {
    let mut writer = writer();
    let _ = writer.set_color(ColorSpec::new().set_bold(true));
    let _ = write!(writer, "built {page_count} page");
    if page_count != 1 {
        let _ = write!(writer, "s");
    }
    if endpoint_count > 0 {
        let _ = write!(writer, " and {endpoint_count} endpoint");
        if endpoint_count != 1 {
            let _ = write!(writer, "s");
        }
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
    emit_summary(
        outcome.outputs.len(),
        outcome.endpoints.len(),
        outcome.elapsed,
    );
}

struct ProgressLayer;

struct ProgressStarted(Instant);

impl<S> Layer<S> for ProgressLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn enabled(&self, metadata: &tracing::Metadata<'_>, _context: Context<'_, S>) -> bool {
        metadata.target() == "aster::build"
    }

    fn on_new_span(&self, attributes: &Attributes<'_>, id: &Id, context: Context<'_, S>) {
        let Some(span) = context.span(id) else {
            return;
        };
        span.extensions_mut()
            .insert(ProgressStarted(Instant::now()));
        emit_progress_start(attributes.metadata().name());
    }

    fn on_close(&self, id: Id, context: Context<'_, S>) {
        let Some(span) = context.span(&id) else {
            return;
        };
        let elapsed = span
            .extensions()
            .get::<ProgressStarted>()
            .map(|start| start.0.elapsed());
        if let Some(elapsed) = elapsed {
            emit_progress_finish(elapsed);
        }
    }
}

fn emit_progress_start(label: &str) {
    let mut writer = writer();
    let _ = writer.set_color(
        ColorSpec::new()
            .set_fg(Some(termcolor::Color::Cyan))
            .set_bold(true),
    );
    let _ = write!(writer, "{label:<10}");
    let _ = writer.reset();
    let _ = write!(writer, " ...");
    let _ = writer.flush();
}

fn emit_progress_finish(elapsed: Duration) {
    let mut writer = writer();
    let _ = writeln!(writer, " {}", format_duration(elapsed));
}

fn format_duration(duration: Duration) -> String {
    if duration < Duration::from_millis(1) {
        "<1ms".into()
    } else if duration < Duration::from_secs(1) {
        format!("{}ms", duration.as_millis())
    } else {
        format!("{:.2}s", duration.as_secs_f64())
    }
}
