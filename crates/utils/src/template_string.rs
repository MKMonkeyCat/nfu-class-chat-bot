use chrono::DateTime;
use serde::Serialize;
use serde_json::{Value, to_value};

pub struct CompiledTemplate {
    nodes: Vec<Node>,
}

#[derive(Clone)]
enum Node {
    Literal(String),
    Variable {
        path: Vec<String>,
        fmt: Option<String>,
        filters: Vec<String>,
    },
    IfBlock {
        path: Vec<String>,
        children: Vec<Node>,
    },
    EachBlock {
        path: Vec<String>,
        children: Vec<Node>,
    },
}

impl CompiledTemplate {
    pub fn compile(template: &str) -> Self {
        let (nodes, _) = Self::parse(template, 0);
        Self { nodes }
    }

    fn parse(template: &str, mut i: usize) -> (Vec<Node>, usize) {
        let mut nodes = Vec::new();
        let bytes = template.as_bytes();
        let mut literal_start = i;

        while i < bytes.len() {
            if bytes[i] == b'{' {
                if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                    i += 2;
                    continue;
                }

                if literal_start < i {
                    nodes.push(Node::Literal(
                        template[literal_start..i]
                            .replace("{{", "{")
                            .replace("}}", "}"),
                    ));
                }

                if let Some(rel_end) = bytes[i + 1..].iter().position(|&b| b == b'}') {
                    let end = i + 1 + rel_end;
                    let token = template[i + 1..end].trim();

                    if token.starts_with("#if ") {
                        let path = token[4..]
                            .trim()
                            .split('.')
                            .map(|s| s.to_string())
                            .collect();
                        let (children, new_i) = Self::parse(template, end + 1);
                        nodes.push(Node::IfBlock { path, children });
                        i = new_i;
                        literal_start = i;
                        continue;
                    }

                    if token == "/if" {
                        return (nodes, end + 1);
                    }

                    if token.starts_with("#each ") {
                        let path = token[6..]
                            .trim()
                            .split('.')
                            .map(|s| s.to_string())
                            .collect();
                        let (children, new_i) = Self::parse(template, end + 1);
                        nodes.push(Node::EachBlock { path, children });
                        i = new_i;
                        literal_start = i;
                        continue;
                    }

                    if token == "/each" {
                        return (nodes, end + 1);
                    }

                    let mut parts = token.split('|');
                    let main = parts.next().unwrap().trim();

                    let filters = parts.map(|s| s.trim().to_string()).collect();

                    let (key, fmt) = match main.split_once(':') {
                        Some((k, f)) => (k.trim(), Some(f.trim().to_string())),
                        None => (main, None),
                    };

                    nodes.push(Node::Variable {
                        path: key.split('.').map(|s| s.to_string()).collect(),
                        fmt,
                        filters,
                    });

                    i = end + 1;
                    literal_start = i;
                    continue;
                }
            }

            i += 1;
        }

        if literal_start < template.len() {
            nodes.push(Node::Literal(
                template[literal_start..]
                    .replace("{{", "{")
                    .replace("}}", "}"),
            ));
        }

        (nodes, i)
    }

    pub fn render<T>(&self, context: &T) -> String
    where
        T: Serialize,
    {
        let json = to_value(context).unwrap_or(Value::Null);
        Self::render_nodes(&self.nodes, &json)
    }

    fn render_nodes(nodes: &[Node], ctx: &Value) -> String {
        let mut out = String::new();

        for node in nodes {
            match node {
                Node::Literal(s) => out.push_str(s),
                Node::Variable { path, fmt, filters } => {
                    if let Some(v) = Self::resolve(ctx, path) {
                        let mut current_val = v.clone();

                        for f in filters {
                            current_val = Self::apply_value_filter(current_val, f);
                        }

                        if let Some(val_str) = Self::format_value(&current_val, fmt.as_deref()) {
                            out.push_str(&val_str);
                        }
                    }
                }
                Node::IfBlock { path, children } => {
                    if let Some(v) = Self::resolve(ctx, path) {
                        if Self::is_truthy(v) {
                            out.push_str(&Self::render_nodes(children, ctx));
                        }
                    }
                }
                Node::EachBlock { path, children } => {
                    if let Some(Value::Array(arr)) = Self::resolve(ctx, path) {
                        for item in arr {
                            out.push_str(&Self::render_nodes(children, item));
                        }
                    }
                }
            }
        }

        out
    }

    fn resolve<'a>(ctx: &'a Value, path: &[String]) -> Option<&'a Value> {
        let mut current = ctx;
        for p in path {
            current = current.get(p)?;
        }
        Some(current)
    }

    fn format_value(v: &Value, fmt: Option<&str>) -> Option<String> {
        match v {
            Value::String(s) => {
                if let Some(f) = fmt {
                    if f.starts_with('%') {
                        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                            return Some(dt.format(f).to_string());
                        }
                    }
                }
                Some(s.clone())
            }
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }

    fn apply_value_filter(val: Value, filter: &str) -> Value {
        let filter_cmd = filter.trim();

        if filter_cmd.starts_with("join") {
            if let Value::Array(arr) = val {
                let delimiter = filter_cmd
                    .strip_prefix("join")
                    .unwrap_or("")
                    .trim()
                    .trim_matches(|c| c == '\'' || c == '"');

                let joined: Vec<String> = arr
                    .iter()
                    .filter_map(|v| match v {
                        Value::String(s) => Some(s.clone()),
                        Value::Number(n) => Some(n.to_string()),
                        Value::Bool(b) => Some(b.to_string()),
                        _ => None,
                    })
                    .collect();

                return Value::String(joined.join(delimiter));
            }
        }

        if let Value::String(s) = val {
            let new_str = match filter_cmd {
                "upper" => s.to_uppercase(),
                "lower" => s.to_lowercase(),
                "trim" => s.trim().to_string(),
                "len" => s.len().to_string(),
                _ => s,
            };
            return Value::String(new_str);
        }

        val
    }

    fn is_truthy(v: &Value) -> bool {
        match v {
            Value::Bool(b) => *b,
            Value::Number(n) => n.as_i64().unwrap_or(0) != 0,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Null => false,
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[derive(Serialize)]
    struct User {
        name: String,
    }

    #[derive(Serialize)]
    struct Ctx {
        name: String,
        is_active: bool,
        users: Vec<User>,
        timestamp: DateTime<Utc>,
        tags: Vec<String>,
    }

    fn sample() -> Ctx {
        Ctx {
            name: "Alice".into(),
            is_active: true,
            users: vec![
                User { name: "Bob".into() },
                User {
                    name: "Carol".into(),
                },
            ],
            timestamp: Utc.with_ymd_and_hms(2026, 5, 20, 13, 14, 0).unwrap(),
            tags: vec!["Rust".into(), "Template".into(), "Engine".into()],
        }
    }

    #[test]
    fn test_variable() {
        let tpl = CompiledTemplate::compile("Hello {name}");
        assert_eq!(tpl.render(&sample()), "Hello Alice");
    }

    #[test]
    fn test_filter() {
        let tpl = CompiledTemplate::compile("{name|upper}");
        assert_eq!(tpl.render(&sample()), "ALICE");
    }

    #[test]
    fn test_if() {
        let tpl = CompiledTemplate::compile("{#if is_active}ACTIVE{/if}");
        assert_eq!(tpl.render(&sample()), "ACTIVE");
    }

    #[test]
    fn test_each() {
        let tpl = CompiledTemplate::compile("{#each users}{name} {/each}");
        assert_eq!(tpl.render(&sample()), "Bob Carol ");
    }

    #[test]
    fn test_date() {
        let tpl = CompiledTemplate::compile("{timestamp:%Y-%m-%d}");
        assert_eq!(tpl.render(&sample()), "2026-05-20");
    }

    #[test]
    fn test_join_filter() {
        let tpl = CompiledTemplate::compile("Tags: {tags|join ', '}");
        assert_eq!(tpl.render(&sample()), "Tags: Rust, Template, Engine");
    }
}
