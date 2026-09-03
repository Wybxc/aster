use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

mod cli;

use crate::cli::{build, dev, init, telemetry, watch};

#[derive(Parser)]
#[command(name = "aster", version, about = "Aster build system")]
struct Cli {
    /// Show detailed build progress; repeat to include ordinary resources
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)]
    verbosity: u8,
    /// Expose development mode to Typst; defaults to true for `dev` only
    #[arg(
        short = 'D',
        long = "dev",
        global = true,
        action = clap::ArgAction::Set,
        num_args = 0..=1,
        default_missing_value = "true",
        value_name = "BOOL"
    )]
    dev: Option<bool>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Create a new Aster project
    Init {
        /// Directory to initialize. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
    },
    /// Build the project
    Build {
        /// Project root directory (containing aster.toml).
        /// Defaults to the nearest ancestor with aster.toml from cwd.
        #[arg(short = 'p', long = "project")]
        project_dir: Option<std::path::PathBuf>,
    },
    /// Build and serve the project with automatic browser refresh
    Dev {
        /// Project root directory (containing aster.toml).
        /// Defaults to the nearest ancestor with aster.toml from cwd.
        #[arg(short = 'p', long = "project")]
        project_dir: Option<std::path::PathBuf>,
        /// Address on which to serve the project.
        #[arg(long, default_value = "127.0.0.1")]
        host: std::net::IpAddr,
        /// Port on which to serve the project.
        #[arg(long, default_value_t = 4321)]
        port: u16,
    },
    /// Build the project and rebuild when its inputs change
    Watch {
        /// Project root directory (containing aster.toml).
        /// Defaults to the nearest ancestor with aster.toml from cwd.
        #[arg(short = 'p', long = "project")]
        project_dir: Option<std::path::PathBuf>,
    },
}

impl Cli {
    fn dev_mode(&self) -> bool {
        self.dev
            .unwrap_or(matches!(&self.command, Commands::Dev { .. }))
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    telemetry::init(cli.verbosity);
    match run(cli) {
        Ok(exit) => exit,
        Err(error) => {
            tracing::error!(
                error = %format_args!("{error:#}"),
                "command failed: {error:#}"
            );
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let dev = cli.dev_mode();
    match cli.command {
        Commands::Init { path } => init::run(path)?,
        Commands::Build { project_dir } => build::run(project_dir, dev)?,
        Commands::Dev {
            project_dir,
            host,
            port,
        } => dev::run(project_dir, host, port, dev)?,
        Commands::Watch { project_dir } => watch::run(project_dir, dev)?,
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind;

    use super::*;

    #[test]
    fn parses_global_verbosity_after_the_subcommand() {
        let cli = Cli::try_parse_from(["aster", "build", "-vv"]).unwrap();
        assert_eq!(cli.verbosity, 2);
    }

    #[test]
    fn keeps_claps_version_flags() {
        for flag in ["-V", "--version"] {
            let error = Cli::try_parse_from(["aster", flag])
                .err()
                .expect("version flag should stop command parsing");
            assert_eq!(error.kind(), ErrorKind::DisplayVersion);
        }
    }

    #[test]
    fn development_mode_defaults_follow_the_command() {
        for (command, expected) in [("build", false), ("watch", false), ("dev", true)] {
            let cli = Cli::try_parse_from(["aster", command]).unwrap();
            assert_eq!(cli.dev_mode(), expected, "unexpected default for {command}");
        }
    }

    #[test]
    fn development_mode_can_be_overridden() {
        let build = Cli::try_parse_from(["aster", "build", "--dev"]).unwrap();
        assert!(build.dev_mode());

        let dev = Cli::try_parse_from(["aster", "dev", "--dev=false"]).unwrap();
        assert!(!dev.dev_mode());
    }
}
