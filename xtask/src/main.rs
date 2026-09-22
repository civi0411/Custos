use anyhow::Result;
use clap::{Parser, Subcommand};
use xshell::{cmd, Shell};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check the workspace for formatting and linting
    Check,
    /// Run end-to-end tests
    TestE2e,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let sh = Shell::new()?;

    match cli.command {
        Commands::Check => {
            cmd!(sh, "cargo fmt --all -- --check").run()?;
            cmd!(sh, "cargo clippy --workspace -- -D warnings").run()?;
        }
        Commands::TestE2e => {
            cmd!(sh, "cargo test --test e2e").run()?;
        }
    }

    Ok(())
}
