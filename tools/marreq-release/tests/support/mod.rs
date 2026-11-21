use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use marreq_release::{
    changelog,
    registry::{COMPONENTS, Component},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

pub const BASE: &str = "0.1.0";
pub const NEXT: &str = "0.1.1";

pub struct Fixture {
    _temp: TempDir,
    pub root: PathBuf,
    pub evidence: PathBuf,
    pub legacy: String,
    pub bootstrap: String,
    records: Value,
}

impl Fixture {
    pub fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("repo");
        let evidence = temp.path().join("evidence");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&evidence).unwrap();
        let mut fixture = Self {
            _temp: temp,
            root,
            evidence,
            legacy: String::new(),
            bootstrap: String::new(),
            records: json!({}),
        };
        fixture.git(&["init", "-q"]);
        for (key, value) in [
            ("user.name", "Release Test"),
            ("user.email", "release@example.test"),
            ("commit.gpgsign", "false"),
            ("tag.gpgsign", "false"),
            ("core.autocrlf", "false"),
        ] {
            fixture.git(&["config", key, value]);
        }
        fixture.write(
            "scripts/release-impact-map.json",
            include_str!("../../../../scripts/release-impact-map.json"),
        );
        for directory in ["marreq-server", "marreq-cloud"] {
            fixture.write(format!("{directory}/Cargo.toml"), format!("[package]\nname = \"{directory}\"\nversion = '{BASE}' # preserve this comment\n"));
        }
        fixture.write(
            "Cargo.lock",
            format!(
                r#"version = 4

[[package]]
name = "marreq-cloud"
version = "{BASE}"

[[package]]
name = "marreq-server"
version = "{BASE}"
dependencies = ["example"]

[[package]]
name = "example"
version = "1.2.3"
source = "registry+https://example.test/index"
"#
            ),
        );
        for directory in ["frontend", "mcp-server"] {
            let package = json!({"name": format!("marreq-{directory}"), "version": BASE, "dependencies": {"example": "^2.0.0"}});
            fixture.write_json(format!("{directory}/package.json"), &package);
            fixture.write_json(
                format!("{directory}/package-lock.json"),
                &json!({
                    "name": format!("marreq-{directory}"), "version": BASE, "lockfileVersion": 3,
                    "packages": {"": package, "node_modules/example": {"version": "2.0.1"}}
                }),
            );
        }
        fixture.write("frontend/src/example.ts", "published frontend\n");
        fixture.legacy = fixture.commit();
        fixture.set_versions(&[], NEXT);
        for name in [
            None,
            Some(Component::Server),
            Some(Component::Cloud),
            Some(Component::Frontend),
            Some(Component::Mcp),
        ] {
            fixture.write(changelog::path(name), format!("# Changelog\n\n## [Unreleased]\n\n## [{BASE}] - Bootstrap\n\n- Initial registry baseline.\n"));
        }
        fixture.bootstrap = fixture.commit();
        for component in COMPONENTS {
            fixture.publish(component, &fixture.legacy.clone(), BASE);
        }
        fixture.write_evidence(None);
        fixture
    }

    pub fn git(&self, args: &[&str]) -> String {
        let result = Command::new("git")
            .current_dir(&self.root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        String::from_utf8(result.stdout).unwrap().trim().to_owned()
    }

    pub fn write(&self, path: impl AsRef<Path>, text: impl AsRef<[u8]>) {
        let target = self.root.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, text).unwrap();
    }

    pub fn read(&self, path: impl AsRef<Path>) -> String {
        fs::read_to_string(self.root.join(path)).unwrap()
    }

    pub fn write_json(&self, path: impl AsRef<Path>, value: &Value) {
        self.write(
            path,
            format!("{}\n", serde_json::to_string_pretty(value).unwrap()),
        );
    }

    pub fn commit(&self) -> String {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", "Fixture"]);
        self.git(&["rev-parse", "HEAD"])
    }

    pub fn reset(&self) {
        self.git(&["reset", "--hard", &self.bootstrap]);
    }

    pub fn command(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_marreq-release"))
            .env_remove("MARREQ_RELEASE_ROOT")
            .current_dir(&self.root)
            .args(args)
            .output()
            .unwrap()
    }

    pub fn call(&self, args: &[&str], error: Option<&str>) -> Value {
        let output = self.command(&[&["--format", "json"], args].concat());
        assert_eq!(
            output.status.code(),
            Some(if error.is_some() { 2 } else { 0 }),
            "args: {args:?}, stdout: {}, stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        if let Some(error) = error {
            assert_eq!(report["status"], "invalid");
            assert!(
                report["error"].as_str().unwrap().contains(error),
                "{report}"
            );
        } else {
            assert_eq!(report["status"], "ok");
        }
        report
    }

    pub fn set_versions(&self, bumps: &[&str], version: &str) {
        let field = |name| if bumps.contains(&name) { version } else { BASE };
        let mut text = format!(
            "[marreq]\nversion = \"{}\"\n\n[components]\n",
            field("marreq")
        );
        for name in COMPONENTS.map(Component::name) {
            text.push_str(&format!("{name} = \"{}\"\n", field(name)));
        }
        self.write("versions.toml", text);
    }

    pub fn add_entry(&self, component: Option<Component>, version: &str) {
        let path = changelog::path(component);
        let before = self.read(&path);
        self.write(
            path,
            before.replacen(
                "## [Unreleased]\n",
                &format!("## [Unreleased]\n\n## [{version}] - 2026-11-21\n\n- Release fixture.\n"),
                1,
            ),
        );
    }

    pub fn prepare(&self, components: &[Component]) -> String {
        let mut bumps = vec!["marreq"];
        bumps.extend(components.iter().map(|name| name.name()));
        self.set_versions(&bumps, NEXT);
        self.add_entry(None, NEXT);
        for component in components {
            self.add_entry(Some(*component), NEXT);
        }
        self.call(&["sync"], None);
        self.commit()
    }

    pub fn publish(&mut self, name: Component, source: &str, version: &str) {
        let tag = format!("marreq-{name}-v{version}");
        self.git(&["tag", "-f", &tag, source]);
        let mut bundle = json!({"format": "marreq.contract.v2", "component": name,
            "version": version, "source_sha": source, "contract": {}});
        if name.is_backend() {
            bundle["component"] = json!("backend");
            bundle["variant"] = json!(name);
        }
        let file = format!("{name}.json");
        let bytes = serde_json::to_vec(&bundle).unwrap();
        self.write(self.evidence.join(&file), &bytes);
        let image_name = match name {
            Component::Frontend => "marreq-frontend",
            Component::Mcp => "marreq-mcp-server",
            _ => name.directory(),
        };
        self.records[name.name()] = json!({"version": version, "source_sha": source, "source_tag": tag,
            "image": format!("ghcr.io/marreqsw/{image_name}"), "digest": format!("sha256:{}", "a".repeat(64)),
            "contract": {"file": file, "sha256": hex::encode(Sha256::digest(bytes))}});
    }

    pub fn write_evidence(&self, product: Option<&str>) {
        for (variant, frontend, mcp) in [
            ("server", "frontend", "mcp-server"),
            ("cloud", "frontend-cloud", "mcp-cloud"),
        ] {
            let mut manifest = json!({"format": "marreq.deployment-provenance.v1", "variant": variant,
                "product_release": product, "components": {}});
            for (name, service) in [
                (variant, format!("marreq-{variant}")),
                ("frontend", frontend.into()),
                ("mcp", mcp.into()),
            ] {
                let mut artifact = self.records[name].clone();
                artifact["service"] = json!(service);
                manifest["components"][name] = artifact;
            }
            self.write_json(
                self.evidence.join(format!("{variant}-provenance.json")),
                &manifest,
            );
        }
    }

    pub fn release(&self, head: &str, error: Option<&str>) -> Value {
        self.release_from(head, &self.bootstrap, &format!("v{NEXT}"), error)
    }

    pub fn release_from(
        &self,
        head: &str,
        previous: &str,
        tag: &str,
        error: Option<&str>,
    ) -> Value {
        let server = self.evidence.join("server-provenance.json");
        let cloud = self.evidence.join("cloud-provenance.json");
        self.call(
            &[
                "release",
                "--head",
                head,
                "--tag",
                tag,
                "--previous",
                previous,
                "--server-provenance",
                server.to_str().unwrap(),
                "--cloud-provenance",
                cloud.to_str().unwrap(),
            ],
            error,
        )
    }

    pub fn impact(&self, base: &str, head: &str) -> Value {
        let result = self.command(&["impact", "--format", "json", "--base", base, "--head", head]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        serde_json::from_slice(&result.stdout).unwrap()
    }
}
