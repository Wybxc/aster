use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "aster", version, about = "Aster build system")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build the project
    Build,
}
