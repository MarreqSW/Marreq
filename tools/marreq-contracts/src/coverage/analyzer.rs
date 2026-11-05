//! Transport recognition, navigation and fail-closed consumer call analysis.
use super::*;
use super::{routes::RouteIndex, syntax::*, transports};
use anyhow::bail;
use tree_sitter::Node;

fn navigation(node: Node<'_>, source: &str) -> bool {
    let object = node.child_by_field_name("object").map(|n| text(n, source));
    match name(node, source).as_str() {
        "assign" | "replace" | "href" => {
            object.is_some_and(|s| s == "location" || s.ends_with(".location"))
        }
        "location" => object == Some("window"),
        "open" => object == Some("window"),
        _ => false,
    }
}

fn api_navigation(path: &str) -> bool {
    path.starts_with("/api/") || path.starts_with("/oauth/") || path.starts_with("/.well-known/")
}

pub(super) fn analyze_file(
    source: &str,
    tsx: bool,
    declared: &BTreeMap<Operation, Value>,
    transport: bool,
    source_path: Option<&str>,
) -> Result<Coverage> {
    let tree = parse(source, tsx)?;
    let root = tree.root_node();
    let routes = RouteIndex::new(declared);
    let mut calls = vec![];
    descend(root, "call_expression", &mut calls);
    let mut result = Coverage {
        operations: BTreeSet::new(),
        call_sites: 0,
    };
    let aliases = transports::aliases(root, source)?;
    for call in calls {
        let fun = field(call, "function")?;
        let is_navigation = navigation(fun, source);
        let fun_name = name(fun, source);
        let fun_name = if is_navigation {
            "fetch".into()
        } else {
            aliases.get(&fun_name).cloned().unwrap_or(fun_name)
        };
        anyhow::ensure!(
            fun_name != "sendBeacon",
            "sendBeacon at {} needs an explicit contract transport adapter",
            location(call)
        );
        if !transports::known(&fun_name) {
            let args = call
                .child_by_field_name("arguments")
                .map(children)
                .unwrap_or_default();
            let mut endpoints = vec![];
            for argument in &args {
                endpoints.extend(values(*argument, source, root, &BTreeMap::new(), 0));
                if argument.kind() == "object" {
                    for pair in children(*argument)
                        .into_iter()
                        .filter(|n| n.kind() == "pair")
                    {
                        if pair
                            .child_by_field_name("key")
                            .is_some_and(|n| matches!(text(n, source), "url" | "path" | "uri"))
                            && let Some(value) = pair.child_by_field_name("value")
                        {
                            endpoints.extend(values(value, source, root, &BTreeMap::new(), 0));
                        }
                    }
                }
            }
            if endpoints.iter().any(|p| p.contains("/api/"))
                && !matches!(
                    fun_name.as_str(),
                    "base" | "project" | "useCases" | "withQuery"
                )
            {
                bail!(
                    "unclassified use of an API URL by {fun_name} at line {}; add an explicit transport adapter",
                    call.start_position().row + 1
                );
            }
            continue;
        }
        // Exactly the shared transports forward arbitrary paths. Their callers
        // are checked here; adding another forwarding layer is not auto-excluded.
        if transport && fun_name == "fetch" {
            let owner = enclosing(call)
                .child_by_field_name("name")
                .map(|n| text(n, source));
            let args = children(field(call, "arguments")?);
            let forwarded = args.first().is_some_and(|arg| {
                let expression = text(*arg, source);
                (expression == "path" && owner != Some("request"))
                    || (expression == "url"
                        && owner == Some("request")
                        && !values(*arg, source, root, &BTreeMap::new(), 0)
                            .iter()
                            .any(|path| path.contains("/api/")))
            });
            if forwarded
                && matches!(
                    owner,
                    Some("request" | "fetchJson" | "fetchBlob" | "fetchDownload")
                )
            {
                continue;
            }
        }
        if fun_name == "download" {
            let mut defs = vec![];
            descend(root, "function_declaration", &mut defs);
            let forwards = defs
                .into_iter()
                .filter(|d| {
                    d.child_by_field_name("name")
                        .is_some_and(|n| text(n, source) == "download")
                })
                .any(|d| {
                    let mut inner = vec![];
                    descend(d, "call_expression", &mut inner);
                    inner.into_iter().any(|c| {
                        c.child_by_field_name("function")
                            .is_some_and(|f| name(f, source) == "fetchDownload")
                    })
                });
            if !forwards {
                continue;
            }
        }
        let args = children(field(call, "arguments")?);
        let Some(path) = args.first() else {
            bail!("HTTP call has no path")
        };
        let mut paths = values(*path, source, root, &BTreeMap::new(), 0);
        if is_navigation {
            // These existing flows navigate to user-selected UI paths or the
            // external provider URL returned by the audited identity-link API.
            // Neither is a new, statically chosen application HTTP operation.
            let explicit_flow = (source_path == Some("frontend/src/pages/LoginPage.tsx")
                && text(*path, source) == "returnTo")
                || (source_path == Some("frontend/src/pages/AccountPage.tsx")
                    && text(*path, source).starts_with("await startIdentityLink("));
            if paths == ["*"] && explicit_flow {
                continue;
            }
            if paths.iter().all(|p| p != "*" && !api_navigation(p)) {
                continue;
            }
        }
        // A forwarding download helper is covered at each concrete call site.
        if fun_name == "fetchDownload"
            && paths == ["*"]
            && enclosing(call)
                .child_by_field_name("name")
                .is_some_and(|n| text(n, source) == "download")
        {
            continue;
        }
        paths = paths
            .into_iter()
            .map(|p| p.split('?').next().unwrap().to_owned())
            .collect();
        paths.sort();
        paths.dedup();
        let method = transports::methods(
            if fun_name == "download" || is_navigation {
                None
            } else {
                args.get(1).copied()
            },
            source,
        )
        .with_context(|| format!("line {}", call.start_position().row + 1))?;
        for path in paths {
            if !api_navigation(&path) {
                bail!(
                    "unresolved HTTP dependency at line {}: {}",
                    call.start_position().row + 1,
                    text(*args.first().unwrap(), source)
                );
            }
            for method in &method {
                result.operations.extend(
                    routes
                        .require(method, &path)
                        .with_context(|| format!("line {}", call.start_position().row + 1))?,
                );
            }
        }
        result.call_sites += 1;
    }
    // Browser navigation to fixed API paths is also a consumer dependency.
    let mut attributes = vec![];
    descend(root, "jsx_attribute", &mut attributes);
    for attribute in attributes {
        let parts = children(attribute);
        if parts.first().is_none_or(|n| text(*n, source) != "href") {
            continue;
        }
        let Some(value) = parts.get(1) else {
            continue;
        };
        let value = if value.kind() == "jsx_expression" {
            children(*value).first().copied().context("empty href")?
        } else {
            *value
        };
        if source_path == Some("frontend/src/components/StatementText.tsx")
            && text(value, source) == "n.href"
        {
            continue;
        }
        for path in values(value, source, root, &BTreeMap::new(), 0) {
            let path = path.split('?').next().unwrap();
            if path == "*" {
                bail!(
                    "unresolved browser navigation at line {}",
                    attribute.start_position().row + 1
                );
            }
            if !api_navigation(path) {
                continue;
            }
            result.operations.extend(
                routes.require("get", path).with_context(|| {
                    format!("href at line {}", attribute.start_position().row + 1)
                })?,
            );
        }
    }
    let mut assignments = vec![];
    descend(root, "assignment_expression", &mut assignments);
    for assignment in assignments {
        if !navigation(field(assignment, "left")?, source) {
            continue;
        }
        for path in values(
            field(assignment, "right")?,
            source,
            root,
            &BTreeMap::new(),
            0,
        ) {
            let path = path.split('?').next().unwrap();
            anyhow::ensure!(
                path != "*",
                "unresolved browser navigation assignment at {}",
                location(assignment)
            );
            if api_navigation(path) {
                result
                    .operations
                    .extend(routes.require("get", path).with_context(|| {
                        format!("navigation at line {}", assignment.start_position().row + 1)
                    })?);
            }
        }
    }
    Ok(result)
}
