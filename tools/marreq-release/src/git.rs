use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, ensure};

pub struct Repository {
    root: PathBuf,
}

impl Repository {
    pub fn new(root: &Path) -> Result<Self> {
        Ok(Self {
            root: root.canonicalize()?,
        })
    }

    pub fn discover() -> Result<Self> {
        let result = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()?;
        ensure!(
            result.status.success(),
            "run marreq-release from a Git worktree"
        );
        #[cfg(unix)]
        let path = {
            use std::os::unix::ffi::OsStrExt;
            let bytes = result.stdout.strip_suffix(b"\n").unwrap_or(&result.stdout);
            Path::new(std::ffi::OsStr::from_bytes(bytes))
        };
        #[cfg(not(unix))]
        let path = Path::new(std::str::from_utf8(&result.stdout)?.trim_end_matches(['\r', '\n']));
        Self::new(path)
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn git(&self, args: &[&str]) -> Result<Vec<u8>> {
        let result = Command::new("git")
            .arg("--no-replace-objects")
            .args(args)
            .current_dir(&self.root)
            .output()?;
        ensure!(
            result.status.success(),
            "Git command failed: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        );
        Ok(result.stdout)
    }

    pub(crate) fn resolve(&self, reference: &str) -> Result<String> {
        let revision = format!("{reference}^{{commit}}");
        let result = self
            .git(&["rev-parse", "--verify", "--end-of-options", &revision])
            .with_context(|| format!("cannot resolve commit {reference:?}"))?;
        Ok(String::from_utf8(result)?.trim().to_owned())
    }

    pub(crate) fn ancestor(&self, base: &str, head: &str) -> Result<()> {
        self.git(&["merge-base", "--is-ancestor", base, head])
            .with_context(|| {
                format!("{base} must be an ancestor of {head}; fetch complete history")
            })?;
        Ok(())
    }

    pub(crate) fn mode(&self, commit: &str, path: &str) -> Result<Option<String>> {
        let output = self.git(&["ls-tree", "--full-tree", commit, "--", path])?;
        if output.is_empty() {
            return Ok(None);
        }
        let mode = output
            .split(|byte| *byte == b' ')
            .next()
            .unwrap_or_default();
        Ok(Some(std::str::from_utf8(mode)?.to_owned()))
    }

    pub(crate) fn committed(&self, commit: &str, path: &str) -> Result<String> {
        let mode = self.mode(commit, path)?;
        ensure!(
            matches!(mode.as_deref(), Some("100644" | "100755")),
            "{commit}:{path}: expected a regular Git blob"
        );
        Ok(String::from_utf8(
            self.git(&["show", &format!("{commit}:{path}")])?,
        )?)
    }

    pub(crate) fn worktree(&self, path: &str) -> Result<String> {
        let target = self.root.join(path);
        ensure!(
            target.canonicalize().with_context(|| path.to_owned())? == target && target.is_file(),
            "{path}: expected a regular repository file"
        );
        fs::read_to_string(target).with_context(|| path.to_owned())
    }

    pub(crate) fn read(&self, commit: Option<&str>, path: &str) -> Result<String> {
        match commit {
            Some(commit) => self.committed(commit, path),
            None => self.worktree(path),
        }
    }
}
