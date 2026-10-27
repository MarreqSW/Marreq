use clap::{Parser, Subcommand};
use marreq_contracts::{artifact, contract, coverage, traffic};
use std::path::PathBuf;
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Action,
}
#[derive(Subcommand)]
enum Action {
    Coverage {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "docs/developer/openapi.yaml")]
        spec: PathBuf,
        #[arg(long)]
        component: String,
    },
    Validate {
        #[arg(long, default_value = "docs/developer/openapi.yaml")]
        spec: PathBuf,
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        document_only: bool,
    },
    Bundle {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        component: String,
        #[arg(long)]
        version: semver::Version,
        #[arg(long)]
        variant: Option<String>,
        #[arg(long)]
        output: PathBuf,
    },
    Verify {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long,requires_all=["candidate_version","candidate_source_sha","candidate_bundle"])]
        candidate_component: Option<String>,
        #[arg(long)]
        candidate_version: Option<semver::Version>,
        #[arg(long)]
        candidate_source_sha: Option<String>,
        #[arg(long)]
        candidate_bundle: Option<PathBuf>,
    },
    VerifyTraffic {
        transcript: PathBuf,
        #[arg(long, default_value = "docs/developer/openapi.yaml")]
        spec: PathBuf,
    },
    Select {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        component: String,
        #[arg(long, default_value = "docs/developer/openapi.yaml")]
        spec: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        allow_unverified: bool,
    },
    Compare {
        consumer: PathBuf,
        backend: PathBuf,
        #[arg(long)]
        variant: String,
    },
}
fn run() -> anyhow::Result<serde_json::Value> {
    match Cli::parse().command {
        Action::Coverage {
            root,
            spec,
            component,
        } => Ok(serde_json::to_value(coverage::scan(
            &root,
            &component,
            &contract::load(&spec)?,
        )?)?),
        Action::Bundle {
            root,
            component,
            version,
            variant,
            output,
        } => {
            let bundle = artifact::export(&root, &component, version, variant)?;
            contract::write(&output, &bundle)?;
            Ok(
                serde_json::json!({"file":output,"sha256":artifact::hash(&std::fs::read(&output)?),"source_sha":bundle.source_sha,"version":bundle.version}),
            )
        }
        Action::Verify {
            manifest,
            candidate_component,
            candidate_version,
            candidate_source_sha,
            candidate_bundle,
        } => {
            let any = candidate_component.is_some()
                || candidate_version.is_some()
                || candidate_source_sha.is_some()
                || candidate_bundle.is_some();
            let all = candidate_component.is_some()
                && candidate_version.is_some()
                && candidate_source_sha.is_some()
                && candidate_bundle.is_some();
            anyhow::ensure!(
                !any || all,
                "candidate verification requires component, version, source SHA and source-exported bundle together"
            );
            let candidate = candidate_component.as_deref().map(|who| {
                (
                    who,
                    candidate_version.as_ref().unwrap(),
                    candidate_source_sha.as_deref().unwrap(),
                    candidate_bundle.as_deref().unwrap(),
                )
            });
            artifact::verify(&manifest, candidate)
        }
        Action::VerifyTraffic { transcript, spec } => {
            traffic::verify(&contract::load(&spec)?, &transcript)
        }
        Action::Select {
            root,
            component,
            spec,
            output,
            allow_unverified,
        } => {
            let spec = contract::load(&spec)?;
            let coverage = coverage::scan(&root, &component, &spec)?;
            contract::write(
                &output,
                &contract::select(&spec, &coverage.operations, None, !allow_unverified)?,
            )?;
            Ok(
                serde_json::json!({"component":component,"operations":coverage.operations.len(),"output":output}),
            )
        }
        Action::Validate {
            spec,
            root,
            document_only,
        } => {
            let spec = contract::load(&spec)?;
            contract::validate(&spec)?;
            for dir in ["frontend", "mcp-server"] {
                let p: artifact::Policy = serde_json::from_value(contract::load(
                    &root.join(dir).join("compatibility.json"),
                )?)?;
                p.validate()?;
            }
            let range: artifact::FrontendRange = serde_json::from_value(contract::load(
                &root.join("marreq-core/frontend_compatibility.json"),
            )?)?;
            anyhow::ensure!(
                range.min_version.cmp_precedence(&range.max_version).is_le(),
                "invalid backend frontend compatibility range"
            );
            let mut counts = serde_json::Map::new();
            if !document_only {
                for who in ["frontend", "mcp"] {
                    let c = coverage::scan(&root, who, &spec)?;
                    let selected = contract::select(&spec, &c.operations, None, true)?;
                    contract::validate_wire_contract(&selected)?;
                    counts.insert(who.into(), serde_json::json!(c.operations.len()));
                }
            }
            Ok(serde_json::json!({"valid":true,"consumer_operations":counts}))
        }
        Action::Compare {
            consumer,
            backend,
            variant,
        } => {
            let findings = contract::compare(
                &contract::load(&consumer)?,
                &contract::load(&backend)?,
                &variant,
            )?;
            Ok(serde_json::json!({"compatible":true,"findings":findings}))
        }
    }
}
fn main() {
    match run() {
        Ok(value) => println!("{}", serde_json::to_string_pretty(&value).unwrap()),
        Err(e) => {
            println!(
                "{}",
                serde_json::json!({"compatible":false,"error":format!("{e:#}")})
            );
            std::process::exit(1);
        }
    }
}
