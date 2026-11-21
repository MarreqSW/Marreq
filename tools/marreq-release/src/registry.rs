use std::{fmt, str::FromStr};

use anyhow::{Context, Result, ensure};
use semver::Version;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub const COMPONENTS: [Component; 4] = [
    Component::Server,
    Component::Cloud,
    Component::Frontend,
    Component::Mcp,
];
pub const BACKENDS: [Component; 2] = [Component::Server, Component::Cloud];

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum Component {
    Server,
    Cloud,
    Frontend,
    Mcp,
}

impl Component {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Server => "server",
            Self::Cloud => "cloud",
            Self::Frontend => "frontend",
            Self::Mcp => "mcp",
        }
    }

    pub const fn directory(self) -> &'static str {
        match self {
            Self::Server => "marreq-server",
            Self::Cloud => "marreq-cloud",
            Self::Frontend => "frontend",
            Self::Mcp => "mcp-server",
        }
    }

    pub const fn is_backend(self) -> bool {
        matches!(self, Self::Server | Self::Cloud)
    }
}

impl fmt::Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StableVersion(Version);

impl FromStr for StableVersion {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self> {
        let version = Version::parse(text)
            .with_context(|| format!("expected stable SemVer X.Y.Z, got {text:?}"))?;
        ensure!(
            version.pre.is_empty() && version.build.is_empty(),
            "expected stable SemVer X.Y.Z, got {text:?}"
        );
        const MAX_SAFE: u64 = (1 << 53) - 1;
        ensure!(
            [version.major, version.minor, version.patch]
                .into_iter()
                .all(|field| field <= MAX_SAFE),
            "SemVer field exceeds npm's numeric limit: {text}"
        );
        Ok(Self(version))
    }
}

impl fmt::Display for StableVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for StableVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for StableVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    marreq: Product,
    components: Components,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Product {
    version: StableVersion,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Components {
    server: StableVersion,
    cloud: StableVersion,
    frontend: StableVersion,
    mcp: StableVersion,
}

impl Registry {
    pub fn parse(text: &str) -> Result<Self> {
        toml::from_str(text).context("versions.toml")
    }

    pub fn product(&self) -> &StableVersion {
        &self.marreq.version
    }

    pub fn component(&self, component: Component) -> &StableVersion {
        match component {
            Component::Server => &self.components.server,
            Component::Cloud => &self.components.cloud,
            Component::Frontend => &self.components.frontend,
            Component::Mcp => &self.components.mcp,
        }
    }

    pub(crate) fn neutral() -> Self {
        let version = StableVersion(Version::new(0, 0, 0));
        Self {
            marreq: Product {
                version: version.clone(),
            },
            components: Components {
                server: version.clone(),
                cloud: version.clone(),
                frontend: version.clone(),
                mcp: version,
            },
        }
    }
}

impl Serialize for Registry {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Versions<'a> {
            marreq: &'a StableVersion,
            #[serde(flatten)]
            components: &'a Components,
        }

        Versions {
            marreq: self.product(),
            components: &self.components,
        }
        .serialize(serializer)
    }
}
