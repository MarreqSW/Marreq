//! Recognized HTTP boundaries, aliases and explicitly unsupported transports.
use super::{syntax::*, *};
use anyhow::bail;
use tree_sitter::Node;

pub(super) fn known(name: &str) -> bool {
    matches!(
        name,
        "fetch" | "fetchJson" | "fetchBlob" | "fetchDownload" | "request" | "download"
    )
}

pub(super) fn methods(init: Option<Node<'_>>, source: &str) -> Result<Vec<String>> {
    let Some(init) = init else {
        return Ok(vec!["get".into()]);
    };
    if init.kind() == "call_expression" {
        let fun = name(field(init, "function")?, source);
        if fun == "write" {
            let args = children(field(init, "arguments")?);
            if let Some(n) = args.first().filter(|n| n.kind() == "string") {
                return Ok(vec![
                    text(*n, source)[1..text(*n, source).len() - 1].to_lowercase(),
                ]);
            }
        }
        bail!("unresolved request options: {}", text(init, source));
    }
    if init.kind() != "object" {
        bail!(
            "request options need an explicit HTTP method: {}",
            text(init, source)
        );
    }
    anyhow::ensure!(
        !children(init).iter().any(|n| n.kind() == "spread_element"),
        "request option spreads need explicit method evidence"
    );
    for p in children(init).into_iter().filter(|n| n.kind() == "pair") {
        if text(field(p, "key")?, source).trim_matches(['\'', '"']) == "method" {
            let v = field(p, "value")?;
            return match v.kind() {
                "string" => Ok(vec![
                    text(v, source)[1..text(v, source).len() - 1].to_lowercase(),
                ]),
                // Both branch methods are required; dynamic arbitrary methods fail.
                "ternary_expression" => ["consequence", "alternative"]
                    .into_iter()
                    .map(|f| {
                        let n = field(v, f)?;
                        if n.kind() != "string" {
                            bail!("unresolved method")
                        };
                        Ok(text(n, source)[1..text(n, source).len() - 1].to_lowercase())
                    })
                    .collect(),
                _ => bail!("unresolved HTTP method: {}", text(v, source)),
            };
        }
    }
    Ok(vec!["get".into()])
}

pub(super) fn aliases(root: Node<'_>, source: &str) -> Result<BTreeMap<String, String>> {
    // Alternate networking implementations need an adapter instead of escaping
    // the shared-transport checks with an unresolved dynamic URL.
    let mut constructors = vec![];
    descend(root, "new_expression", &mut constructors);
    for expression in constructors {
        let constructor = name(field(expression, "constructor")?, source);
        anyhow::ensure!(
            !matches!(
                constructor.as_str(),
                "XMLHttpRequest" | "EventSource" | "WebSocket"
            ),
            "{constructor} at {} needs an explicit contract transport adapter",
            location(expression)
        );
    }
    let mut import_statements = vec![];
    descend(root, "import_statement", &mut import_statements);
    for import in import_statements {
        if text(import, source).starts_with("import type ") {
            continue;
        }
        if let Some(module) = import.child_by_field_name("source") {
            let module = text(module, source).trim_matches(['\'', '"']);
            anyhow::ensure!(
                !matches!(
                    module,
                    "axios"
                        | "ky"
                        | "got"
                        | "undici"
                        | "superagent"
                        | "http"
                        | "https"
                        | "node:http"
                        | "node:https"
                ),
                "network module {module} at {} needs an explicit contract transport adapter",
                location(import)
            );
        }
    }
    let mut aliases = BTreeMap::new();
    let mut imports = vec![];
    descend(root, "import_specifier", &mut imports);
    for import in imports {
        if let (Some(original), Some(alias)) = (
            import.child_by_field_name("name"),
            import.child_by_field_name("alias"),
        ) && known(text(original, source))
        {
            aliases.insert(
                text(alias, source).to_owned(),
                text(original, source).to_owned(),
            );
        }
    }
    let mut variables = vec![];
    descend(root, "variable_declarator", &mut variables);
    for var in variables {
        if let (Some(id), Some(value)) = (
            var.child_by_field_name("name"),
            var.child_by_field_name("value"),
        ) {
            let original = name(value, source);
            let original = aliases.get(&original).cloned().unwrap_or(original);
            if known(&original) {
                aliases.insert(text(id, source).to_owned(), original);
            }
        }
    }
    Ok(aliases)
}
