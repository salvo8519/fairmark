//! `fairmark` — JSON in, JSON out. Designed to be driven by AI coding agents
//! (Claude Code, Codex, ...) as well as by humans and scripts.

use std::io::{ErrorKind, Read, Write};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::json;

#[derive(Parser)]
#[command(
    name = "fairmark",
    version,
    about = "Deterministic valuation engine for analysts and AI agents"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List every method with its version and status (available or preview).
    Methods,
    /// Print the JSON Schema of a method's input (default), of its output with --output, or of
    /// its whole result envelope with --envelope.
    Schema {
        method: String,
        /// Print the schema of the method's output (the `value` of the result).
        #[arg(long, conflicts_with = "envelope")]
        output: bool,
        /// Print the schema of the whole result envelope (method, value, explain, meta).
        #[arg(long)]
        envelope: bool,
    },
    /// Run a method. Input JSON from --input FILE or stdin.
    Run {
        method: String,
        #[arg(short, long)]
        input: Option<String>,
        /// Print only the result value, without explain and meta.
        #[arg(long)]
        value_only: bool,
    },
    /// Re-run a saved result's input and check the hash and value match.
    Verify {
        /// Result file produced by `fairmark run`.
        result: String,
        /// Input file used to produce it.
        #[arg(short, long)]
        input: String,
    },
}

fn main() -> ExitCode {
    match dispatch(Cli::parse().command) {
        Ok(out) => match emit(std::io::stdout().lock(), &out) {
            Ok(()) => ExitCode::SUCCESS,
            // The reader closed the pipe early (`fairmark methods | head -1`):
            // it has all it wanted, so exit quietly with success (issue #191).
            Err(e) if e.kind() == ErrorKind::BrokenPipe => ExitCode::SUCCESS,
            Err(e) => {
                report(&io_error(e));
                ExitCode::FAILURE
            }
        },
        Err(err) => {
            report(&err);
            ExitCode::FAILURE
        }
    }
}

/// Writes `value` as pretty JSON followed by a newline, then flushes, so that
/// every write error (including a closed pipe) is returned instead of the
/// panic of `println!`. The single place where the CLI writes to stdout.
fn emit(mut out: impl Write, value: &serde_json::Value) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(value).expect("serializable");
    out.write_all(text.as_bytes())?;
    out.write_all(b"\n")?;
    out.flush()
}

/// Writes an error report to stderr. A failure to write it (stderr closed)
/// is ignored: the exit status still reports the error.
fn report(err: &serde_json::Value) {
    let _ = emit(std::io::stderr().lock(), err);
}

/// JSON error report of an I/O failure, in the shape the CLI already uses for
/// unreadable input files.
fn io_error(e: std::io::Error) -> serde_json::Value {
    json!({ "error": { "code": "FM-E001", "message": e.to_string() } })
}

fn dispatch(cmd: Command) -> Result<serde_json::Value, serde_json::Value> {
    let core = |e: fairmark_core::FairmarkError| json!({ "error": e.to_report() });

    match cmd {
        Command::Methods => Ok(json!(fairmark_core::METHODS)),
        Command::Schema {
            method,
            output,
            envelope,
        } => if output {
            fairmark_core::output_schema(&method)
        } else if envelope {
            fairmark_core::result_schema(&method)
        } else {
            fairmark_core::input_schema(&method)
        }
        .map_err(core),
        Command::Run {
            method,
            input,
            value_only,
        } => {
            let text = read_input(input.as_deref()).map_err(io_error)?;
            let out = fairmark_core::run(&method, &text).map_err(core)?;
            Ok(if value_only {
                out["value"].clone()
            } else {
                out
            })
        }
        Command::Verify { result, input } => {
            let saved: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&result).map_err(io_error)?)
                    .map_err(
                        |e| json!({ "error": { "code": "FM-E001", "message": e.to_string() } }),
                    )?;
            let method = saved["method"].as_str().unwrap_or_default().to_owned();
            let fresh = fairmark_core::run(&method, &read_input(Some(&input)).map_err(io_error)?)
                .map_err(core)?;
            let same_hash = saved["meta"]["input_hash"] == fresh["meta"]["input_hash"];
            let same_value = saved["value"] == fresh["value"];
            let report = json!({
                "verified": same_hash && same_value,
                "same_input_hash": same_hash,
                "same_value": same_value,
                "engine_version": fresh["meta"]["engine_version"],
            });
            if same_hash && same_value {
                Ok(report)
            } else {
                Err(report)
            }
        }
    }
}

fn read_input(path: Option<&str>) -> std::io::Result<String> {
    match path {
        Some(p) if p != "-" => std::fs::read_to_string(p),
        _ => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
    }
}
