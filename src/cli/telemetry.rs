use std::fmt;
use std::io::IsTerminal;

use anstyle::{AnsiColor, Style};
use aster::BuildOutcome;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::field::RecordFields;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::format::{FmtSpan, FormatEvent, FormatFields, Writer};
use tracing_subscriber::fmt::{FmtContext, FormattedFields};
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;

pub fn init(verbosity: u8) {
    let level = match verbosity {
        0 => LevelFilter::INFO,
        1 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };
    let targets = Targets::new().with_target(env!("CARGO_CRATE_NAME"), level);
    let ansi = std::io::stderr().is_terminal()
        && std::env::var_os("NO_COLOR").is_none_or(|value| value.is_empty());
    let layer = tracing_subscriber::fmt::layer()
        .fmt_fields(SentenceFields { ansi })
        .with_span_events(FmtSpan::CLOSE)
        .event_format(IndentedFormat { ansi })
        .with_writer(std::io::stderr)
        .with_filter(targets);
    let _ = tracing::subscriber::set_global_default(tracing_subscriber::registry().with(layer));
}

pub fn report_build(outcome: &BuildOutcome) {
    for warning in &outcome.warnings {
        tracing::warn!(warning = %warning, "{warning}");
    }
    let pages = outcome.outputs.len();
    let endpoints = outcome.endpoints.len();
    let routes = if endpoints == 0 {
        format!("{pages} page{}", if pages == 1 { "" } else { "s" })
    } else {
        format!(
            "{pages} page{} and {endpoints} endpoint{}",
            if pages == 1 { "" } else { "s" },
            if endpoints == 1 { "" } else { "s" }
        )
    };
    tracing::info!(
        pages,
        endpoints,
        elapsed = ?outcome.elapsed,
        "built {routes} in {:.1}s",
        outcome.elapsed.as_secs_f64()
    );
}

struct SentenceFields {
    ansi: bool,
}

impl<'writer> FormatFields<'writer> for SentenceFields {
    fn format_fields<R: RecordFields>(&self, writer: Writer<'writer>, fields: R) -> fmt::Result {
        let mut visitor = SentenceVisitor {
            writer,
            ansi: self.ansi,
            message: None,
            parts: Vec::new(),
        };
        fields.record(&mut visitor);
        visitor.finish()
    }
}

struct SentenceVisitor<'writer> {
    writer: Writer<'writer>,
    ansi: bool,
    message: Option<String>,
    parts: Vec<(&'static str, String)>,
}

impl SentenceVisitor<'_> {
    fn finish(mut self) -> fmt::Result {
        if let Some(message) = self.message {
            write!(self.writer, "{message}")?;
        }
        for (name, value) in self.parts {
            match name {
                "route" | "source" | "template" | "project" | "address" => {
                    write!(self.writer, " ")?;
                    write_accent(&mut self.writer, &value, self.ansi)?;
                }
                "output" => {
                    write!(self.writer, " with output ")?;
                    write_accent(&mut self.writer, &value, self.ansi)?;
                }
                "destination" => {
                    write!(self.writer, " to ")?;
                    write_accent(&mut self.writer, &value, self.ansi)?;
                }
                "language" => {
                    write!(self.writer, " ")?;
                    write_emphasis(&mut self.writer, &value, self.ansi)?;
                }
                "tool" => {
                    write!(self.writer, " with ")?;
                    write_emphasis(&mut self.writer, &value, self.ansi)?;
                }
                "time.busy" => {
                    write!(self.writer, " in ")?;
                    write_emphasis(&mut self.writer, &value, self.ansi)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl Visit for SentenceVisitor<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "message" if value != "close" => self.message = Some(value.into()),
            "message" => {}
            name => self.parts.push((name, value.into())),
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let value = format!("{value:?}");
        if field.name() == "message" {
            self.message = Some(value);
        } else {
            self.parts.push((field.name(), value));
        }
    }
}

fn write_accent(writer: &mut Writer<'_>, value: &str, ansi: bool) -> fmt::Result {
    if ansi {
        let style = Style::new().fg_color(Some(AnsiColor::Green.into()));
        write!(writer, "{style}{value}{style:#}")
    } else {
        write!(writer, "{value}")
    }
}

fn write_emphasis(writer: &mut Writer<'_>, value: &str, ansi: bool) -> fmt::Result {
    if ansi {
        let style = Style::new().bold();
        write!(writer, "{style}{value}{style:#}")
    } else {
        write!(writer, "{value}")
    }
}

struct IndentedFormat {
    ansi: bool,
}

impl<S, N> FormatEvent<S, N> for IndentedFormat
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        context: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        write_level(&mut writer, event.metadata().level(), self.ansi)?;

        let lifecycle = event.metadata().is_span();
        let scope_depth = context
            .event_scope()
            .map(|scope| scope.count())
            .unwrap_or_default();
        let depth = scope_depth.saturating_sub(usize::from(lifecycle));
        write!(writer, "{:width$}", "", width = depth * 2)?;

        if lifecycle {
            let span = context
                .event_scope()
                .and_then(|mut scope| scope.next())
                .expect("span lifecycle event has its originating span");
            let extensions = span.extensions();
            match extensions.get::<FormattedFields<N>>() {
                Some(fields) if !fields.is_empty() => write!(writer, "{fields}")?,
                _ => write!(writer, "{}", span.name())?,
            }
        }

        context
            .field_format()
            .format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

fn write_level(writer: &mut Writer<'_>, level: &Level, ansi: bool) -> fmt::Result {
    let (label, color) = match *level {
        Level::TRACE => ("TRACE", AnsiColor::Magenta),
        Level::DEBUG => ("DEBUG", AnsiColor::Blue),
        Level::INFO => ("INFO ", AnsiColor::Green),
        Level::WARN => ("WARN ", AnsiColor::Yellow),
        Level::ERROR => ("ERROR", AnsiColor::Red),
    };
    if ansi {
        let style = Style::new().fg_color(Some(color.into())).bold();
        write!(writer, "{style}{label}{style:#} ")
    } else {
        write!(writer, "{label} ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_styles_are_scoped_to_their_values() {
        let mut output = String::new();
        let mut writer = Writer::new(&mut output);

        write_accent(&mut writer, "/pages/index.typ", true).unwrap();
        write!(writer, " ").unwrap();
        write_emphasis(&mut writer, "12ms", true).unwrap();

        assert_eq!(output, "\x1b[32m/pages/index.typ\x1b[0m \x1b[1m12ms\x1b[0m");
    }

    #[test]
    fn semantic_styles_fall_back_to_plain_text() {
        let mut output = String::new();
        let mut writer = Writer::new(&mut output);

        write_accent(&mut writer, "/pages/index.typ", false).unwrap();
        write!(writer, " ").unwrap();
        write_emphasis(&mut writer, "12ms", false).unwrap();

        assert_eq!(output, "/pages/index.typ 12ms");
    }
}
