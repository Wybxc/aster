use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "aster", version, about = "Aster build system")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the project
    Build,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build => {
            println!("Building...");
        }
    }
}
