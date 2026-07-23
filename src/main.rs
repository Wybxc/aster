mod cli;
mod compile;
mod project;
mod world;

use clap::Parser;
use termcolor::{ColorChoice, StandardStream};
use typst_kit::diagnostics::DiagnosticFormat;

use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build => {
            let cwd = std::env::current_dir().unwrap_or_else(|e| {
                eprintln!("error: failed to get current directory: {e}");
                std::process::exit(1);
            });

            let root = project::find_root(&cwd).unwrap_or_else(|| {
                eprintln!("error: no aster.toml found in current or parent directories");
                std::process::exit(1);
            });

            let src_dir = root.join("src");
            if !src_dir.is_dir() {
                eprintln!("error: src/ directory not found in project");
                std::process::exit(1);
            }

            let entries = project::find_typ_files(&src_dir);
            if entries.is_empty() {
                eprintln!("error: no .typ files found in src/");
                std::process::exit(1);
            }

            let mut has_errors = false;

            for entry in &entries {
                let relative = entry.strip_prefix(&src_dir).expect("file must be under src/");
                let output = root.join("dist").join(relative).with_extension("html");

                match compile::run(entry, &root) {
                    Ok(html) => {
                        if let Some(parent) = output.parent() {
                            std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                                eprintln!(
                                    "error: failed to create directory {}: {e}",
                                    parent.display()
                                );
                                std::process::exit(1);
                            });
                        }
                        std::fs::write(&output, &html).unwrap_or_else(|e| {
                            eprintln!("error: failed to write {}: {e}", output.display());
                            std::process::exit(1);
                        });
                    }
                    Err(err) => {
                        has_errors = true;
                        let relative = relative.with_extension("html");
                        eprintln!("error: failed to build {}", relative.display());
                        let mut writer = StandardStream::stderr(ColorChoice::Auto);
                        if typst_kit::diagnostics::emit(
                            &mut writer,
                            &err.world,
                            &err.diagnostics,
                            DiagnosticFormat::Human,
                        )
                        .is_err()
                        {
                            for diag in &err.diagnostics {
                                eprintln!("error: {diag:?}");
                            }
                        }
                    }
                }
            }

            if has_errors {
                std::process::exit(1);
            }
        }
    }
}
