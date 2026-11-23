use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use marreq_release::{Repository, impact, sync, validation};
use serde_json::{Value, json};

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}

#[derive(Parser)]
#[command(
    about = "Check the release registry, synchronize versions, or validate a release candidate"
)]
struct Cli {
    #[arg(long, value_enum, default_value = "text", global = true)]
    format: Format,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check registry, package metadata and changelogs.
    Check {
        #[arg(long)]
        head: Option<String>,
        #[arg(long)]
        base: Option<String>,
    },
    /// Synchronize mirrored package version fields.
    Sync {
        /// Verify synchronization without writing.
        #[arg(long)]
        check: bool,
    },
    /// Check the proposed product tag spelling.
    Tag {
        tag: String,
        #[arg(long)]
        head: Option<String>,
    },
    /// Validate a committed release against trusted publication evidence.
    Release {
        #[arg(long)]
        head: String,
        #[arg(long)]
        tag: String,
        #[arg(long)]
        previous: String,
        #[arg(long)]
        server_provenance: Option<PathBuf>,
        #[arg(long)]
        cloud_provenance: Option<PathBuf>,
    },
    /// Classify committed changes with the candidate's release impact map.
    Impact {
        #[arg(long)]
        base: String,
        #[arg(long)]
        head: String,
    },
}

fn repository() -> Result<Repository> {
    match std::env::var_os("MARREQ_RELEASE_ROOT") {
        Some(root) => Repository::new(Path::new(&root)),
        None => Repository::discover(),
    }
}

fn run(command: Command) -> Result<(Value, String)> {
    let repo = repository()?;
    let message = "Release versions validated".to_owned();
    match command {
        Command::Check { head, base } => {
            let (versions, bootstrap) = validation::check(&repo, head.as_deref(), base.as_deref())?;
            let mut result = json!({"status": "ok", "versions": versions});
            if bootstrap {
                result["bootstrap"] = json!(true);
            }
            Ok((result, message))
        }
        Command::Sync { check } => {
            let (versions, paths) = sync::synchronize(&repo, check)?;
            let message = if check {
                message
            } else if paths.is_empty() {
                "Synchronized: already up to date".to_owned()
            } else {
                format!("Synchronized: {}", paths.join(", "))
            };
            Ok((
                json!({"status": "ok", "versions": versions, "updated_paths": paths}),
                message,
            ))
        }
        Command::Tag { tag, head } => {
            let versions = validation::tag(&repo, &tag, head.as_deref())?;
            Ok((json!({"status": "ok", "versions": versions}), message))
        }
        Command::Release {
            head,
            tag,
            previous,
            server_provenance,
            cloud_provenance,
        } => {
            let (versions, head, decisions) = validation::release(
                &repo,
                &head,
                &tag,
                &previous,
                [server_provenance.as_deref(), cloud_provenance.as_deref()],
            )?;
            Ok((
                json!({"status": "ok", "versions": versions, "head": head, "components": decisions}),
                message,
            ))
        }
        Command::Impact { .. } => unreachable!("impact has a separate report format"),
    }
}

fn quote_path(path: &str) -> String {
    format!(
        "'{}'",
        path.escape_debug()
            .to_string()
            .replace("\\\"", "\"")
            .replace('\'', "\\'")
    )
}

fn print_impact(base: &str, head: &str, format: Format) -> ExitCode {
    let report = match repository().and_then(|repo| impact::detect(&repo, base, head)) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("Release impact: {error:#}");
            return ExitCode::from(2);
        }
    };
    match format {
        Format::Json => println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("serializable impact report")
        ),
        Format::Text => {
            println!("{} -> {}", report.base_sha, report.head_sha);
            for (dimension, entries) in [
                ("artifacts", &report.artifacts),
                ("checks", &report.checks),
                ("infrastructure", &report.infrastructure),
            ] {
                println!("{dimension}:");
                if entries.is_empty() {
                    println!("  none");
                }
                for (name, paths) in entries {
                    println!(
                        "  {name}: {}",
                        paths
                            .iter()
                            .map(|path| quote_path(path))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
            }
        }
    }
    if report.unknown_inputs.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!("Unclassified or unsupported release inputs:");
        for path in &report.unknown_inputs {
            eprintln!("  {}", quote_path(path));
        }
        ExitCode::from(2)
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Command::Impact { base, head } = cli.command {
        return print_impact(&base, &head, cli.format);
    }
    match run(cli.command) {
        Ok((report, message)) => {
            match cli.format {
                Format::Json => println!(
                    "{}",
                    serde_json::to_string_pretty(&report).expect("serializable release report")
                ),
                Format::Text => println!("{message}"),
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            match cli.format {
                Format::Json => println!(
                    "{}",
                    json!({"status": "invalid", "error": format!("{error:#}")})
                ),
                Format::Text => eprintln!("Release versions: {error:#}"),
            }
            ExitCode::from(2)
        }
    }
}
