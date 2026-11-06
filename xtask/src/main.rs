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
    /// Run canonical quality gates (Desktop build, Canvas Smoke, Architecture Invariants, Workspace Check)
    Gate,
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
        Commands::Gate => {
            println!("==> [1/4] Desktop UI Build & Typecheck");
            cmd!(sh, "npm --prefix ui/desktop run build").run()?;
            println!("==> [2/4] Canvas Smoke Invariants");
            cmd!(sh, "node --test tools/canvas-smoke.mjs").run()?;
            println!("==> [3/4] Architecture Invariants & Contracts");
            cmd!(
                sh,
                "cargo test -p custos-testkit -p custos-tests-contract --offline"
            )
            .run()?;
            println!("==> [4/4] Workspace Cargo Check");
            cmd!(sh, "cargo check --workspace --all-targets --offline").run()?;
            println!("==> All Quality Gates Passed cleanly!");
        }
    }

    Ok(())
}
