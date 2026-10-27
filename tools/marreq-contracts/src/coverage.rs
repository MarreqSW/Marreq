//! Conservative TypeScript call analysis using the real TS/TSX grammar.
//! Unknown URL or HTTP-method expressions fail instead of dropping dependencies.
use crate::contract::{Operation, operations};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    path::Path,
};
use tree_sitter::Node;

#[derive(Debug, Serialize)]
pub struct Coverage {
    pub operations: BTreeSet<Operation>,
    pub call_sites: usize,
}

fn children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}
fn descend<'a>(node: Node<'a>, kind: &str, found: &mut Vec<Node<'a>>) {
    if node.kind() == kind {
        found.push(node);
    }
    for child in children(node) {
        descend(child, kind, found);
    }
}
fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}
fn field<'a>(node: Node<'a>, name: &str) -> Result<Node<'a>> {
    node.child_by_field_name(name).context("missing AST field")
}
fn name(node: Node<'_>, source: &str) -> String {
    if matches!(
        node.kind(),
        "await_expression" | "parenthesized_expression" | "instantiation_expression"
    ) && let Some(inner) = children(node).first()
    {
        return name(*inner, source);
    }
    if node.kind() == "subscript_expression"
        && let Some(index) = node
            .child_by_field_name("index")
            .filter(|n| n.kind() == "string")
    {
        return text(index, source).trim_matches(['\'', '"']).into();
    }
    node.child_by_field_name("property")
        .map_or_else(|| text(node, source), |n| text(n, source))
        .to_owned()
}
fn enclosing(mut node: Node<'_>) -> Node<'_> {
    while let Some(parent) = node.parent() {
        if matches!(
            node.kind(),
            "function_declaration" | "method_definition" | "arrow_function"
        ) {
            return node;
        }
        node = parent;
    }
    node
}
fn combine(left: Vec<String>, right: Vec<String>) -> Vec<String> {
    left.into_iter()
        .flat_map(|a| right.iter().map(move |b| format!("{a}{b}")))
        .collect()
}

// Only finite string construction is evaluated. Unknown parameter values become
// a segment wildcard; an entirely unknown endpoint or HTTP method is rejected.
fn values(
    node: Node<'_>,
    source: &str,
    root: Node<'_>,
    bound: &BTreeMap<String, Vec<String>>,
    depth: usize,
) -> Vec<String> {
    if depth > 16 {
        return vec!["*".into()];
    }
    let eval = |n| values(n, source, root, bound, depth + 1);
    match node.kind() {
        "string" => vec![text(node, source)[1..text(node, source).len() - 1].to_owned()],
        "template_string" => {
            let mut result = vec![String::new()];
            for child in children(node) {
                let part = if child.kind() == "string_fragment" {
                    vec![text(child, source).to_owned()]
                } else if child.kind() == "template_substitution" {
                    children(child)
                        .first()
                        .map_or(vec!["*".into()], |n| eval(*n))
                } else {
                    vec![text(child, source).to_owned()]
                };
                result = combine(result, part);
                if result.len() > 128 {
                    return vec!["*".into()];
                }
            }
            result
        }
        "ternary_expression" => ["consequence", "alternative"]
            .into_iter()
            .filter_map(|f| node.child_by_field_name(f))
            .flat_map(eval)
            .collect(),
        "binary_expression"
            if node
                .child_by_field_name("operator")
                .is_some_and(|n| text(n, source) == "+") =>
        {
            combine(
                eval(field(node, "left").unwrap()),
                eval(field(node, "right").unwrap()),
            )
        }
        "identifier" => {
            let id = text(node, source);
            if let Some(v) = bound.get(id) {
                return v.clone();
            }
            let scope = enclosing(node);
            let mut vars = vec![];
            descend(scope, "variable_declarator", &mut vars);
            if let Some(value) = vars
                .into_iter()
                .filter(|v| {
                    v.start_byte() < node.start_byte()
                        && v.parent()
                            .and_then(|p| p.child(0))
                            .is_some_and(|n| n.kind() == "const")
                })
                .filter(|v| {
                    let mut block = v.parent();
                    while block.is_some_and(|p| !matches!(p.kind(), "statement_block" | "program"))
                    {
                        block = block.and_then(|p| p.parent());
                    }
                    let mut parent = Some(node);
                    while let Some(p) = parent {
                        if Some(p) == block {
                            return true;
                        }
                        parent = p.parent();
                    }
                    false
                })
                .rev()
                .find_map(|v| {
                    (v.child_by_field_name("name")
                        .is_some_and(|n| text(n, source) == id))
                    .then(|| v.child_by_field_name("value"))
                    .flatten()
                })
            {
                return eval(value);
            }
            if let Some(parameters) = scope.child_by_field_name("parameters") {
                for parameter in children(parameters) {
                    let ident = parameter
                        .child_by_field_name("pattern")
                        .or_else(|| parameter.child_by_field_name("name"));
                    let numeric = parameter.child_by_field_name("type").is_some_and(|t| {
                        text(t, source).trim().trim_start_matches(':').trim() == "number"
                    });
                    if numeric && ident.is_some_and(|n| text(n, source) == id) {
                        return vec!["#".into()];
                    }
                }
            }
            vec!["*".into()]
        }
        "number" => vec![text(node, source).into()],
        "member_expression"
            if node.child_by_field_name("property").is_some_and(|n| {
                let property = text(n, source);
                property == "id" || property.ends_with("Id") || property.ends_with("_id")
            }) =>
        {
            vec!["#".into()]
        }
        "parenthesized_expression" | "as_expression" | "non_null_expression" => children(node)
            .first()
            .map_or(vec!["*".into()], |n| eval(*n)),
        "call_expression" => {
            let Ok(fun) = field(node, "function") else {
                return vec!["*".into()];
            };
            let fun_name = name(fun, source);
            let args = node
                .child_by_field_name("arguments")
                .map(children)
                .unwrap_or_default();
            if matches!(fun_name.as_str(), "encodeURIComponent" | "String") {
                return vec!["*".into()];
            }
            if fun_name == "withQuery" {
                return args.first().map_or(vec!["*".into()], |n| eval(*n));
            }
            let mut defs = vec![];
            descend(root, "function_declaration", &mut defs);
            descend(root, "method_definition", &mut defs);
            // Arrow-valued helpers such as `const base = (projectId) => ...`.
            let mut vars = vec![];
            descend(root, "variable_declarator", &mut vars);
            for var in vars {
                if var
                    .child_by_field_name("name")
                    .is_some_and(|n| text(n, source) == fun_name)
                    && let Some(v) = var
                        .child_by_field_name("value")
                        .filter(|n| n.kind() == "arrow_function")
                {
                    defs.push(v);
                }
            }
            for def in defs {
                if def.kind() != "arrow_function"
                    && !def
                        .child_by_field_name("name")
                        .is_some_and(|n| text(n, source) == fun_name)
                {
                    continue;
                }
                let mut scope = bound.clone();
                if let Some(parameters) = def.child_by_field_name("parameters") {
                    for (i, p) in children(parameters).into_iter().enumerate() {
                        let id = p
                            .child_by_field_name("pattern")
                            .or_else(|| p.child_by_field_name("name"))
                            .unwrap_or(p);
                        let default = p.child_by_field_name("value");
                        scope.insert(
                            text(id, source).into(),
                            args.get(i).map_or_else(
                                || default.map_or(vec!["*".into()], eval),
                                |n| eval(*n),
                            ),
                        );
                    }
                }
                if let Some(body) = def.child_by_field_name("body") {
                    let mut returns = vec![];
                    if body.kind() == "statement_block" {
                        descend(body, "return_statement", &mut returns);
                    } else {
                        returns.push(body);
                    }
                    let result: Vec<_> = returns
                        .into_iter()
                        .flat_map(|r| {
                            let v = if r.kind() == "return_statement" {
                                children(r).first().copied()
                            } else {
                                Some(r)
                            };
                            v.map_or(vec![], |v| values(v, source, root, &scope, depth + 1))
                        })
                        .collect();
                    if !result.is_empty() {
                        return result;
                    }
                }
            }
            vec!["*".into()]
        }
        _ => vec!["*".into()],
    }
}

fn methods(init: Option<Node<'_>>, source: &str) -> Result<Vec<String>> {
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

// Intersection of segment globs. '*' never crosses '/', so a collection call
// cannot accidentally require every nested resource beneath it.
pub fn intersects(a: &str, b: &str) -> bool {
    fn go(a: &[u8], b: &[u8], i: usize, j: usize, seen: &mut HashSet<(usize, usize)>) -> bool {
        if !seen.insert((i, j)) {
            return false;
        }
        if i == a.len() && j == b.len() {
            return true;
        }
        // '#' is a nonempty numeric identifier. It keeps a typed id lookup
        // from accidentally selecting a reserved sibling such as /index_status.
        match (a.get(i), b.get(j)) {
            (Some(b'#'), Some(b'#')) => return go(a, b, i + 1, j + 1, seen),
            (Some(b'#'), Some(b'*')) => {
                return go(a, b, i + 1, j, seen) || go(a, b, i, j + 1, seen);
            }
            (Some(b'*'), Some(b'#')) => {
                return go(a, b, i, j + 1, seen) || go(a, b, i + 1, j, seen);
            }
            (Some(b'#'), Some(d)) if d.is_ascii_digit() => {
                return go(a, b, i + 1, j + 1, seen) || go(a, b, i, j + 1, seen);
            }
            (Some(d), Some(b'#')) if d.is_ascii_digit() => {
                return go(a, b, i + 1, j + 1, seen) || go(a, b, i + 1, j, seen);
            }
            (Some(b'#'), _) | (_, Some(b'#')) => return false,
            _ => (),
        }
        if a.get(i) == Some(&b'*') {
            return go(a, b, i + 1, j, seen)
                || (j < b.len() && b[j] != b'/' && go(a, b, i, j + 1, seen));
        }
        if b.get(j) == Some(&b'*') {
            return go(a, b, i, j + 1, seen)
                || (i < a.len() && a[i] != b'/' && go(a, b, i + 1, j, seen));
        }
        i < a.len() && j < b.len() && a[i] == b[j] && go(a, b, i + 1, j + 1, seen)
    }
    go(a.as_bytes(), b.as_bytes(), 0, 0, &mut HashSet::new())
}
fn path_glob(path: &str) -> String {
    let mut out = String::new();
    let mut param = false;
    for c in path.chars() {
        if c == '{' {
            param = true;
            out.push('*');
        } else if c == '}' {
            param = false;
        } else if !param {
            out.push(c);
        }
    }
    out
}

pub fn analyze(
    source: &str,
    tsx: bool,
    declared: &BTreeMap<Operation, Value>,
    transport: bool,
) -> Result<Coverage> {
    analyze_file(source, tsx, declared, transport, false)
}

fn analyze_file(
    source: &str,
    tsx: bool,
    declared: &BTreeMap<Operation, Value>,
    transport: bool,
    user_content_links: bool,
) -> Result<Coverage> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&if tsx {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    } else {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    })?;
    let tree = parser.parse(source, None).context("parse TypeScript")?;
    let root = tree.root_node();
    if root.has_error() {
        let mut nodes = vec![];
        descend(root, "ERROR", &mut nodes);
        // The upstream TSX grammar treats a bare ampersand in JSX text as an
        // error although TypeScript accepts it. No executable expression is
        // inside such a text fragment; every other parse error fails closed.
        let harmless = |n: &Node<'_>| {
            tsx && n.parent().is_some_and(|p| p.kind() == "jsx_element")
                && text(*n, source).starts_with('&')
                && !text(*n, source).contains(['{', '}', '<', '>'])
        };
        if nodes.is_empty() || !nodes.iter().all(harmless) {
            bail!(
                "TypeScript syntax errors prevent reliable coverage analysis: {:?}",
                nodes
                    .iter()
                    .map(|n| (n.start_position().row + 1, text(*n, source)))
                    .collect::<Vec<_>>()
            );
        }
    }
    let mut calls = vec![];
    descend(root, "call_expression", &mut calls);
    let mut result = Coverage {
        operations: BTreeSet::new(),
        call_sites: 0,
    };
    let known = |name: &str| {
        matches!(
            name,
            "fetch" | "fetchJson" | "fetchBlob" | "fetchDownload" | "request" | "download"
        )
    };
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
    for call in calls {
        let fun = field(call, "function")?;
        let fun_name = name(fun, source);
        let fun_name = aliases.get(&fun_name).cloned().unwrap_or(fun_name);
        if !matches!(
            fun_name.as_str(),
            "fetch" | "fetchJson" | "fetchBlob" | "fetchDownload" | "request" | "download"
        ) {
            let args = call
                .child_by_field_name("arguments")
                .map(children)
                .unwrap_or_default();
            let mut endpoints = vec![];
            if let Some(first) = args.first() {
                endpoints.extend(values(*first, source, root, &BTreeMap::new(), 0));
                if first.kind() == "object" {
                    for pair in children(*first).into_iter().filter(|n| n.kind() == "pair") {
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
            if matches!(
                owner,
                Some("request" | "fetchJson" | "fetchBlob" | "fetchDownload")
            ) {
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
        let method = methods(
            if fun_name == "download" {
                None
            } else {
                args.get(1).copied()
            },
            source,
        )
        .with_context(|| format!("line {}", call.start_position().row + 1))?;
        for path in paths {
            if !path.starts_with("/api/") {
                bail!(
                    "unresolved HTTP dependency at line {}: {}",
                    call.start_position().row + 1,
                    text(*args.first().unwrap(), source)
                );
            }
            for method in &method {
                let found: Vec<_> = declared
                    .keys()
                    .filter(|op| op.method == *method && intersects(&path, &path_glob(&op.path)))
                    .cloned()
                    .collect();
                if found.is_empty() {
                    bail!(
                        "undocumented consumer operation: {} {} at line {}",
                        method,
                        path,
                        call.start_position().row + 1
                    );
                }
                result.operations.extend(found);
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
        if user_content_links && text(value, source) == "n.href" {
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
            if !path.starts_with("/api/") {
                continue;
            }
            let keys: Vec<_> = declared
                .keys()
                .filter(|key| key.method == "get" && intersects(path, &path_glob(&key.path)))
                .cloned()
                .collect();
            if keys.is_empty() {
                bail!("undocumented browser API dependency {path}");
            }
            result.operations.extend(keys);
        }
    }
    Ok(result)
}

pub fn scan(root: &Path, component: &str, spec: &Value) -> Result<Coverage> {
    anyhow::ensure!(
        matches!(component, "frontend" | "mcp"),
        "consumer must be frontend or mcp"
    );
    let directory = root.join(if component == "frontend" {
        "frontend/src"
    } else {
        "mcp-server/src"
    });
    let declared = operations(spec, None)?;
    let mut result = Coverage {
        operations: BTreeSet::new(),
        call_sites: 0,
    };
    for entry in walkdir::WalkDir::new(directory) {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type().is_file()
            || !matches!(
                path.extension().and_then(|x| x.to_str()),
                Some("ts" | "tsx")
            )
        {
            continue;
        }
        let relative = path.strip_prefix(root)?.to_string_lossy();
        if relative.contains("/__tests__/")
            || relative.ends_with(".test.ts")
            || relative.ends_with("generated-contract.ts")
        {
            continue;
        }
        let source = std::fs::read_to_string(path)?;
        let transport =
            relative == "frontend/src/api/transport.ts" || relative == "mcp-server/src/client.ts";
        let coverage = analyze_file(
            &source,
            path.extension().is_some_and(|x| x == "tsx"),
            &declared,
            transport,
            relative == "frontend/src/components/StatementText.tsx",
        )
        .with_context(|| relative.to_string())?;
        result.call_sites += coverage.call_sites;
        result.operations.extend(coverage.operations);
    }
    // Protocol navigation/discovery isn't fetch(): explicit, small, tested roots.
    let protocol = if component == "frontend" {
        vec!["auth_auth_external_start", "auth_auth_external_callback"]
    } else {
        vec![
            "oauth_authorization_server_metadata",
            "oauth_protected_resource_metadata_mcp",
            "oauth_register",
            "oauth_authorize_page",
            "oauth_authorize_decision",
            "oauth_token",
        ]
    };
    for id in protocol {
        let key = declared
            .iter()
            .find(|(_, v)| v["operationId"] == id)
            .map(|(k, _)| k.clone())
            .with_context(|| format!("missing protocol operation {id}"))?;
        result.operations.insert(key);
    }
    Ok(result)
}
