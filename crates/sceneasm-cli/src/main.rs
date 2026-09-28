use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use sceneasm_core::{assemble, render, Assembly, Target};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    name = "sceneasm",
    version,
    about = "Scene-first native assembler toolchain"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DiagnosticFormat {
    Text,
    Json,
}

#[derive(Debug, Serialize)]
struct JsonDiagnostic<'a> {
    code: &'a str,
    severity: &'static str,
    message: &'a str,
    file: Option<&'a str>,
    line: Option<usize>,
    column_start: Option<usize>,
    column_end: Option<usize>,
    raster_line: Option<u16>,
    nominal_cycles: Option<u16>,
    stalled_cycles: Option<u16>,
    actual_cycles: Option<u16>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Build {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long, default_value = "c64")]
        target: String,
        /// Write deterministic symbol map (name = $hhhh).
        #[arg(long)]
        symbols: Option<PathBuf>,
    },
    Check {
        input: PathBuf,
        #[arg(long, default_value = "c64")]
        target: String,
        #[arg(long, value_enum, default_value = "text")]
        diagnostic_format: DiagnosticFormat,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Build {
            input,
            output,
            target,
            symbols,
        } => {
            let target = parse_target(&target)?;
            let source = fs::read_to_string(&input)
                .with_context(|| format!("reading {}", input.display()))?;
            let assembly = assemble(&source, target)?;
            print_diagnostics(&assembly);
            if assembly.has_errors() {
                anyhow::bail!(
                    "build aborted: {} error diagnostic(s)",
                    assembly.errors().count()
                );
            }
            let output = output.unwrap_or_else(|| input.with_extension("prg"));

            let mut prg = Vec::with_capacity(assembly.bytes.len() + 2);
            prg.extend_from_slice(&assembly.origin.to_le_bytes());
            prg.extend_from_slice(&assembly.bytes);
            fs::write(&output, prg).with_context(|| format!("writing {}", output.display()))?;

            if let Some(symbols_path) = symbols {
                let mut entries: Vec<_> = assembly.symbols.iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                let mut text = String::new();
                for (name, value) in entries {
                    text.push_str(&format!("{name} = ${value:04x}\n"));
                }
                fs::write(&symbols_path, text)
                    .with_context(|| format!("writing {}", symbols_path.display()))?;
            }

            println!(
                "built {} bytes at ${:04x} -> {}",
                assembly.bytes.len(),
                assembly.origin,
                output.display()
            );
        }
        Command::Check {
            input,
            target,
            diagnostic_format,
        } => {
            let target = parse_target(&target)?;
            let source = fs::read_to_string(&input)
                .with_context(|| format!("reading {}", input.display()))?;
            let assembly = assemble(&source, target)?;
            match diagnostic_format {
                DiagnosticFormat::Text => print_diagnostics(&assembly),
                DiagnosticFormat::Json => print_json_diagnostics(&assembly)?,
            }
            if assembly.has_errors() {
                anyhow::bail!(
                    "check failed: {} error diagnostic(s)",
                    assembly.errors().count()
                );
            }
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

fn print_diagnostics(assembly: &Assembly) {
    for diagnostic in &assembly.diagnostics {
        eprintln!("{}", render::render(diagnostic, &assembly.source_map));
    }
}

fn print_json_diagnostics(assembly: &Assembly) -> Result<()> {
    let diagnostics: Vec<_> = assembly
        .diagnostics
        .iter()
        .map(|diagnostic| {
            let primary = diagnostic.primary.as_ref();
            let timing = diagnostic.timing.as_ref();
            JsonDiagnostic {
                code: diagnostic.code,
                severity: match diagnostic.severity {
                    sceneasm_core::diagnostic::Severity::Error => "error",
                    sceneasm_core::diagnostic::Severity::Warning => "warning",
                    sceneasm_core::diagnostic::Severity::Info => "info",
                    sceneasm_core::diagnostic::Severity::Hint => "hint",
                },
                message: &diagnostic.message,
                file: primary
                    .and_then(|label| assembly.source_map.file(label.span.file_id))
                    .map(|file| file.name.as_str()),
                line: primary.map(|label| label.span.line),
                column_start: primary.map(|label| label.span.column_start),
                column_end: primary.map(|label| label.span.column_end),
                raster_line: timing.map(|timing| timing.raster_line),
                nominal_cycles: timing.map(|timing| timing.nominal_cycles),
                stalled_cycles: timing.map(|timing| timing.stalled_cycles),
                actual_cycles: timing.map(|timing| timing.actual_cycles),
            }
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&diagnostics)?);
    Ok(())
}
