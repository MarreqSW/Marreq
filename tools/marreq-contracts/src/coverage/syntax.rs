//! TypeScript AST access and bounded, finite string-expression evaluation.
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use tree_sitter::Node;

pub(super) fn children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}
pub(super) fn descend<'a>(node: Node<'a>, kind: &str, found: &mut Vec<Node<'a>>) {
    if node.kind() == kind {
        found.push(node);
    }
    for child in children(node) {
        descend(child, kind, found);
    }
}
pub(super) fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}
pub(super) fn field<'a>(node: Node<'a>, name: &str) -> Result<Node<'a>> {
    node.child_by_field_name(name).context("missing AST field")
}
pub(super) fn name(node: Node<'_>, source: &str) -> String {
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
pub(super) fn enclosing(mut node: Node<'_>) -> Node<'_> {
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
pub(super) fn values(
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
            if text(fun, source) == "URL.createObjectURL" {
                return vec!["blob:*".into()];
            }
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

pub(super) fn parse(source: &str, tsx: bool) -> Result<tree_sitter::Tree> {
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
    Ok(tree)
}

pub(super) fn location(node: Node<'_>) -> String {
    let p = node.start_position();
    format!("{}:{}", p.row + 1, p.column + 1)
}
