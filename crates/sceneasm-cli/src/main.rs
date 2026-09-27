use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sceneasm_core::{assemble, Target};

#[derive(Debug, Parser)]
#[command(name = "sceneasm", version, about = "Scene-first native assembler toolchain")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Build {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long, default_value = "c64")]
        target: String,
    },
    Check {
        input: PathBuf,
        #[arg(long, default_value = "c64")]
        target: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Build { input, output, target } => {
            let target = parse_target(&target)?;
            let source = fs::read_to_string(&input)
                .with_context(|| format!("reading {}", input.display()))?;
            let assembly = assemble(&source, target)?;
            let output = output.unwrap_or_else(|| input.with_extension("prg"));

            let mut prg = Vec::with_capacity(assembly.bytes.len() + 2);
            prg.extend_from_slice(&assembly.origin.to_le_bytes());
            prg.extend_from_slice(&assembly.bytes);
            fs::write(&output, prg)
                .with_context(|| format!("writing {}", output.display()))?;

            println!(
                "built {} bytes at ${:04x} -> {}",
                assembly.bytes.len(),
                assembly.origin,
                output.display()
            );
        }
        Command::Check { input, target } => {
            let target = parse_target(&target)?;
            let source = fs::read_to_string(&input)
                .with_context(|| format!("reading {}", input.display()))?;
            let assembly = assemble(&source, target)?;
            println!(
                "ok: {} bytes, origin ${:04x}, {} symbols",
                assembly.bytes.len(),
                assembly.origin,
                assembly.symbols.len()
            );
        }
    }
    Ok(())
}

fn parse_target(name: &str) -> Result<Target> {
    match name {
        "c64" => Ok(Target::c64()),
        other => anyhow::bail!("unsupported target: {other}"),
    }
}
