use std::{borrow::Cow, collections::BTreeMap, sync::LazyLock};

use anyhow::{Context, Result, ensure};
use regex::Regex;
use time::{Date, Month};

use crate::{
    Repository,
    registry::{COMPONENTS, Component, Registry, StableVersion},
};

const LEGACY_HEADING: &str = "## [0.1.0] - 05/03/2026";
const LEGACY_BOOTSTRAP_LINK: &str =
    "  Earlier product notes and accumulated unreleased changes are in [CHANGELOG](../CHANGELOG).";
const LEGACY_CHANGELOG_DIRECTORY: &str = "changelogs";
static HEADING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[ \t]*##(?:[ \t]|$)").expect("heading regex"));
static RELEASE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^## \[([^\]]+)\] - (Bootstrap|[0-9]{4}-[0-9]{2}-[0-9]{2})$")
        .expect("release regex")
});

pub fn path(component: Option<Component>) -> String {
    match component {
        None => "CHANGELOG.md".to_owned(),
        Some(component) => format!("{}/CHANGELOG.md", component.directory()),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    date: Option<Date>,
    notes: String,
}

#[derive(Debug, Default)]
pub(crate) struct History {
    entries: BTreeMap<StableVersion, Entry>,
    archived: Option<String>,
}

#[derive(Clone, Copy)]
enum Section {
    Unreleased,
    Release,
    Archive,
}

impl History {
    pub(crate) fn is_bootstrap(&self, version: &StableVersion) -> bool {
        self.entries
            .get(version)
            .is_some_and(|entry| entry.date.is_none())
    }

    pub(crate) fn is_initial(&self, version: &StableVersion) -> bool {
        self.entries.len() == 1 && self.is_bootstrap(version)
    }

    pub(crate) fn dated(&self, version: &StableVersion) -> bool {
        self.entries
            .get(version)
            .is_some_and(|entry| entry.date.is_some())
    }
}

fn parse_date(text: &str) -> Result<Date> {
    let year = text[..4].parse()?;
    ensure!(year > 0, "year must be positive");
    Ok(Date::from_calendar_date(
        year,
        Month::try_from(text[5..7].parse::<u8>()?)?,
        text[8..].parse()?,
    )?)
}

fn has_notes(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim_start();
        !line.is_empty() && !line.starts_with('#')
    })
}

fn normalize_lines(text: &str) -> Cow<'_, str> {
    // Treat the same separators as the previous validator's splitlines().
    // In particular, Git's CRLF checkout conversion must preserve history.
    const SEPARATORS: [char; 9] = [
        '\r', '\u{000b}', '\u{000c}', '\u{001c}', '\u{001d}', '\u{001e}', '\u{0085}', '\u{2028}',
        '\u{2029}',
    ];
    if text.contains(SEPARATORS) {
        Cow::Owned(text.replace("\r\n", "\n").replace(SEPARATORS, "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

fn parse(text: &str, expected: &StableVersion, product: bool) -> Result<History> {
    let normalized = normalize_lines(text);
    let text = normalized.as_ref();
    let mut history = History::default();
    let mut current = None;
    let mut start = 0;
    let mut previous: Option<(StableVersion, Option<Date>)> = None;

    for (offset, line) in text
        .split_inclusive('\n')
        .scan(0, |offset, line| {
            let start = *offset;
            *offset += line.len();
            Some((start, line.trim_end_matches('\n')))
        })
        .filter(|(_, line)| HEADING.is_match(line))
        .chain(std::iter::once((text.len(), "")))
    {
        let notes = text[start..offset].trim_end_matches('\n');
        match current {
            Some(Section::Release) => {
                let (version, date) = previous.as_ref().context("missing release heading")?;
                ensure!(has_notes(notes), "[{version}] needs release notes");
                history.entries.insert(
                    version.clone(),
                    Entry {
                        date: *date,
                        notes: notes.to_owned(),
                    },
                );
            }
            Some(Section::Archive) => {
                ensure!(
                    has_notes(notes),
                    "archived product version needs release notes"
                );
                history.archived = Some(notes.to_owned());
            }
            _ => {}
        }

        start = if offset == text.len() {
            offset
        } else {
            text[offset..]
                .find('\n')
                .map_or(text.len(), |end| offset + end + 1)
        };
        if line.is_empty() {
            break;
        }
        ensure!(
            history.archived.is_none(),
            "archived product notes must appear last"
        );
        if line == "## [Unreleased]" {
            ensure!(
                current.is_none(),
                "duplicate or misplaced Unreleased heading"
            );
            current = Some(Section::Unreleased);
        } else if product && line == LEGACY_HEADING && matches!(current, Some(Section::Release)) {
            current = Some(Section::Archive);
        } else {
            let fields = RELEASE
                .captures(line)
                .filter(|_| current.is_some())
                .context("expected an Unreleased heading and '## [X.Y.Z] - YYYY-MM-DD'")?;
            let number: StableVersion = fields[1].parse()?;
            let status = &fields[2];
            ensure!(
                !history.entries.contains_key(&number),
                "duplicate version {number}"
            );
            let date = if status == "Bootstrap" {
                None
            } else {
                Some(parse_date(status).with_context(|| format!("invalid release date {status}"))?)
            };
            if let Some((last, date)) = &previous {
                ensure!(*last > number, "versions must descend");
                ensure!(
                    date.is_some(),
                    "only the oldest version may be marked Bootstrap"
                );
            } else {
                ensure!(
                    number == *expected,
                    "latest entry must match registry version {expected}"
                );
            }
            previous = Some((number, date));
            current = Some(Section::Release);
        }
    }
    ensure!(
        !history.entries.is_empty(),
        "latest entry must match registry version {expected}"
    );
    if history.archived.is_some() {
        let archive_version = "0.1.0".parse::<StableVersion>()?;
        ensure!(
            history
                .entries
                .keys()
                .next()
                .is_some_and(|number| *number > archive_version),
            "archive overlaps the product registry"
        );
    }
    Ok(history)
}

pub(crate) type Histories = BTreeMap<Option<Component>, History>;

fn read_history(
    repo: &Repository,
    commit: Option<&str>,
    component: Option<Component>,
    expected: &StableVersion,
) -> Result<History> {
    let path = path(component);
    let text = repo.read(commit, &path)?;
    parse(&text, expected, component.is_none()).with_context(|| path)
}

pub(crate) fn read(
    repo: &Repository,
    commit: Option<&str>,
    registry: &Registry,
) -> Result<Histories> {
    std::iter::once(None)
        .chain(COMPONENTS.map(Some))
        .map(|component| {
            let version =
                component.map_or_else(|| registry.product(), |name| registry.component(name));
            Ok((component, read_history(repo, commit, component, version)?))
        })
        .collect()
}

fn legacy_archive(repo: &Repository, commit: &str) -> Result<Option<String>> {
    if repo.mode(commit, "CHANGELOG")?.is_none() {
        return Ok(None);
    }
    let text = repo.committed(commit, "CHANGELOG")?;
    let text = normalize_lines(&text);
    let (_, notes) = text
        .split_once(&format!("{LEGACY_HEADING}\n"))
        .context("historical CHANGELOG has an unexpected archive layout")?;
    Ok(Some(notes.trim_end_matches('\n').to_owned()))
}

// Historical commits on the original registry branch used a central layout.
// Candidate/worktree reads always require the canonical component paths.
pub(crate) fn previous(repo: &Repository, commit: &str, registry: &Registry) -> Result<Histories> {
    if repo.mode(commit, &path(None))?.is_some() {
        return read(repo, Some(commit), registry);
    }
    let mut histories = Histories::new();
    for component in std::iter::once(None).chain(COMPONENTS.map(Some)) {
        let version = component.map_or_else(|| registry.product(), |name| registry.component(name));
        let name = component.map_or("marreq", Component::name);
        let old_path = format!("{LEGACY_CHANGELOG_DIRECTORY}/{name}.md");
        let old = repo.committed(commit, &old_path)?;
        let mut history = parse(&old, version, false).with_context(|| old_path)?;
        if component.is_none() {
            history.archived = legacy_archive(repo, commit)?;
            if let Some(entry) = history
                .entries
                .values_mut()
                .find(|entry| entry.date.is_none())
                && let Some(notes) = entry.notes.strip_suffix(LEGACY_BOOTSTRAP_LINK)
            {
                entry.notes = notes.trim_end_matches('\n').to_owned();
            }
        }
        histories.insert(component, history);
    }
    Ok(histories)
}

pub(crate) fn preserve_legacy_archive(
    repo: &Repository,
    base: &str,
    current: &Histories,
) -> Result<()> {
    ensure!(
        legacy_archive(repo, base)? == current[&None].archived,
        "marreq: historical legacy changelog notes changed or disappeared"
    );
    Ok(())
}

pub(crate) fn preserve(previous: &Histories, current: &Histories) -> Result<()> {
    for (component, previous) in previous {
        let name = component.map_or("marreq", Component::name);
        let current = &current[component];
        for (version, entry) in &previous.entries {
            ensure!(
                current.entries.get(version) == Some(entry),
                "{name}: historical changelog entry for {version} changed or disappeared"
            );
        }
        ensure!(
            previous.archived == current.archived,
            "{name}: historical legacy changelog notes changed or disappeared"
        );
    }
    Ok(())
}

pub(crate) fn progress(
    previous: &Histories,
    current: &Histories,
    old: &Registry,
    new: &Registry,
) -> Result<()> {
    for (component, previous) in previous {
        let (name, old, new) = match component {
            Some(component) => (
                component.name(),
                old.component(*component),
                new.component(*component),
            ),
            None => ("marreq", old.product(), new.product()),
        };
        ensure!(new >= old, "{name}: version regressed from {old} to {new}");
        let added: Vec<_> = current[component]
            .entries
            .keys()
            .filter(|version| !previous.entries.contains_key(*version))
            .collect();
        let expected = if new == old { vec![] } else { vec![new] };
        ensure!(
            added == expected,
            "{name}: expected only new changelog versions {expected:?}"
        );
    }
    preserve(previous, current)
}

pub(crate) fn retired_path(path: &str) -> bool {
    path.strip_prefix(LEGACY_CHANGELOG_DIRECTORY)
        .and_then(|path| path.strip_prefix('/'))
        .and_then(|path| path.strip_suffix(".md"))
        .is_some_and(|name| {
            name == "marreq" || COMPONENTS.iter().any(|component| component.name() == name)
        })
}
