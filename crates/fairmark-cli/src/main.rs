//! `fairmark` — JSON in, JSON out. Designed to be driven by AI coding agents
//! (Claude Code, Codex, ...) as well as by humans and scripts.

use std::io::Read;
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
    /// Print the JSON Schema of a method's input.
    Schema { method: String },
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
        Ok(out) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&out).expect("serializable")
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&err).expect("serializable")
            );
            ExitCode::FAILURE
        }
    }
}

fn dispatch(cmd: Command) -> Result<serde_json::Value, serde_json::Value> {
    let core = |e: fairmark_core::FairmarkError| json!({ "error": e.to_report() });
    let io =
        |e: std::io::Error| json!({ "error": { "code": "FM-E001", "message": e.to_string() } });

    match cmd {
        Command::Methods => Ok(json!(fairmark_core::METHODS)),
        Command::Schema { method } => fairmark_core::input_schema(&method).map_err(core),
        Command::Run {
            method,
            input,
            value_only,
        } => {
            let text = read_input(input.as_deref()).map_err(io)?;
            let out = fairmark_core::run(&method, &text).map_err(core)?;
            Ok(if value_only {
                out["value"].clone()
            } else {
                out
            })
        }
        Command::Verify { result, input } => {
            let saved: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(&result).map_err(io)?,
            )
            .map_err(|e| json!({ "error": { "code": "FM-E001", "message": e.to_string() } }))?;
            let method = saved["method"].as_str().unwrap_or_default().to_owned();
            let fresh = fairmark_core::run(&method, &read_input(Some(&input)).map_err(io)?)
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
