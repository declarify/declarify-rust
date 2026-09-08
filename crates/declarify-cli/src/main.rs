mod generate;

use clap::{Parser, Subcommand};

/// Declarify: a code-first Rust resource generator.
#[derive(Parser)]
#[command(name = "declarify", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan a package's Rust source for resource declarations and validate
    /// them. Writes no generated files. See `docs/03-declaration-and-generation.md`.
    Validate {
        /// The Cargo package name to scan, e.g. `notes`.
        #[arg(long)]
        package: String,
    },
    /// Validate, render, format, compile-check, and atomically install the
    /// generated tree for a package.
    Generate {
        #[arg(long)]
        package: String,
    },
    /// Exit non-zero if the committed generated tree differs from a fresh
    /// generation. Never writes to the committed tree.
    Check {
        #[arg(long)]
        package: String,
    },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Validate { package } => run_validate(&package),
        Command::Generate { package } => generate::run_generate(&package),
        Command::Check { package } => generate::run_check(&package),
    }
}

fn run_validate(package: &str) -> anyhow::Result<()> {
    let outcome = declarify_schema::validate_package(package)?;

    for resource in &outcome.resources {
        println!(
            "resource `{}` ({}, plural `{}`): spec `{}` ({} field(s)), status `{}` ({} field(s)), \
             {} operation(s)",
            resource.name,
            resource.api_version,
            resource.plural,
            resource.spec.name,
            resource.spec.fields.len(),
            resource.status.name,
            resource.status.fields.len(),
            resource.operations.len(),
        );
    }

    if outcome.diagnostics.is_empty() {
        println!(
            "declarify validate: {} resource(s) valid, no manifest required",
            outcome.resources.len()
        );
        Ok(())
    } else {
        for diagnostic in &outcome.diagnostics {
            eprintln!("error: {diagnostic}");
        }
        anyhow::bail!(
            "declarify validate: {} error(s) in package `{package}`",
            outcome.diagnostics.len()
        );
    }
}
