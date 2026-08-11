use std::io::Write;
use std::net::SocketAddr;
use std::path::Path;
use std::time::{Duration, Instant};

use aster::BuildOutcome;
use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;

fn writer() -> StandardStream {
    StandardStream::stderr(ColorChoice::Auto)
}

pub fn init(verbosity: u8) {
    let subscriber = tracing_subscriber::registry().with(ProgressLayer { verbosity });
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

struct ProgressLayer {
    verbosity: u8,
}

struct ProgressSpan {
    started: Instant,
    depth: usize,
    expanded: bool,
    name: &'static str,
}

#[derive(Default)]
struct DisplayFields {
    detail: Option<String>,
    message: Option<String>,
}

impl Visit for DisplayFields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.record(field, format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.record(field, value.to_owned());
    }
}

impl DisplayFields {
    fn record(&mut self, field: &Field, value: String) {
        match field.name() {
            "detail" => self.detail = Some(value),
            "message" => self.message = Some(value),
            _ => {}
        }
    }
}

impl<S> Layer<S> for ProgressLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn enabled(&self, metadata: &tracing::Metadata<'_>, _context: Context<'_, S>) -> bool {
        let level = match self.verbosity {
            0 => Level::INFO,
            1 => Level::DEBUG,
            _ => Level::TRACE,
        };
        metadata.target() == "aster::build" && metadata.level() <= &level
    }

    fn on_new_span(&self, attributes: &Attributes<'_>, id: &Id, context: Context<'_, S>) {
        let Some(span) = context.span(id) else {
            return;
        };
        let (depth, break_parent) = span
            .parent()
            .and_then(|parent| {
                let mut extensions = parent.extensions_mut();
                let progress = extensions.get_mut::<ProgressSpan>()?;
                let break_parent = !progress.expanded;
                progress.expanded = true;
                Some((progress.depth + 1, break_parent))
            })
            .unwrap_or((0, false));

        let mut fields = DisplayFields::default();
        attributes.record(&mut fields);
        let label = match fields.detail {
            Some(detail) => format!("{} {detail}", attributes.metadata().name()),
            None => attributes.metadata().name().to_owned(),
        };
        span.extensions_mut().insert(ProgressSpan {
            started: Instant::now(),
            depth,
            expanded: false,
            name: attributes.metadata().name(),
        });
        if break_parent {
            emit_progress_break();
        }
        emit_progress_start(&label, depth);
    }

    fn on_event(&self, event: &Event<'_>, context: Context<'_, S>) {
        let mut fields = DisplayFields::default();
        event.record(&mut fields);
        let Some(message) = fields.message else {
            return;
        };
        let Some(span) = context.event_span(event) else {
            return;
        };
        let (depth, break_parent) = {
            let mut extensions = span.extensions_mut();
            let Some(progress) = extensions.get_mut::<ProgressSpan>() else {
                return;
            };
            let break_parent = !progress.expanded;
            progress.expanded = true;
            (progress.depth + 1, break_parent)
        };
        if break_parent {
            emit_progress_break();
        }
        emit_progress_detail(&message, depth);
    }

    fn on_close(&self, id: Id, context: Context<'_, S>) {
        let Some(span) = context.span(&id) else {
            return;
        };
        let progress = span.extensions().get::<ProgressSpan>().map(|progress| {
            (
                progress.started.elapsed(),
                progress.depth,
                progress.expanded,
                progress.name,
            )
        });
        if let Some((elapsed, depth, true, name)) = progress {
            emit_progress_completed(name, elapsed, depth + 1);
        } else if let Some((elapsed, _, false, _)) = progress {
            emit_progress_finish(elapsed);
        }
    }
}

fn emit_progress_start(label: &str, depth: usize) {
    let mut writer = writer();
    let _ = writer.set_color(
        ColorSpec::new()
            .set_fg(Some(termcolor::Color::Cyan))
            .set_bold(true),
    );
    if depth == 0 {
        let _ = write!(writer, "{label:<10}");
    } else {
        let _ = write!(writer, "{:indent$}{label}", "", indent = depth * 2);
    }
    let _ = writer.reset();
    let _ = write!(writer, " ...");
    let _ = writer.flush();
}

fn emit_progress_break() {
    let _ = writeln!(writer());
}

fn emit_progress_detail(message: &str, depth: usize) {
    let mut writer = writer();
    let _ = writeln!(writer, "{:indent$}{message}", "", indent = depth * 2);
}

fn emit_progress_finish(elapsed: Duration) {
    let mut writer = writer();
    let _ = writeln!(writer, " {}", format_duration(elapsed));
}

fn emit_progress_completed(name: &str, elapsed: Duration, depth: usize) {
    let mut writer = writer();
    let _ = writeln!(
        writer,
        "{:indent$}{name} finished in {}",
        "",
        format_duration(elapsed),
        indent = depth * 2
    );
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
