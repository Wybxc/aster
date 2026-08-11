use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

mod cli;

use crate::cli::{build, dev, diag, init, watch};

#[derive(Parser)]
#[command(name = "aster", version, about = "Aster build system")]
struct Cli {
    /// Show detailed build progress; repeat to include ordinary resources
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)]
    verbosity: u8,
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

fn main() -> ExitCode {
    let cli = Cli::parse();
    diag::init(cli.verbosity);
    match run(cli) {
        Ok(exit) => exit,
        Err(error) => {
            diag::emit_error(&format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Commands::Init { path } => init::run(path)?,
        Commands::Build { project_dir } => build::run(project_dir)?,
        Commands::Dev {
            project_dir,
            host,
            port,
        } => dev::run(project_dir, host, port)?,
        Commands::Watch { project_dir } => watch::run(project_dir)?,
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
}
