use serde::Serialize;
use toml::Value;
use utils::{DocNode, DocReader};

fn build_toml_with_comments(node: &DocNode, value: &Value, path: Vec<String>) -> String {
    let mut output = String::new();

    match (node, value) {
        (DocNode::Node(doc, children), Value::Table(map)) => {
            if path.is_empty() && !doc.is_empty() {
                output.push_str(&format!("# {}\n", doc));

                if !children
                    .values()
                    .any(|node| matches!(node, DocNode::Leaf(_)))
                {
                    output.push_str("#");
                }
                output.push_str("\n");
            }

            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.cmp(b));

            let mut leaves = Vec::new();
            let mut complexes = Vec::new();
            for (key, val) in entries {
                let is_complex_array = if let Value::Array(arr) = val {
                    arr.iter().any(|v| v.is_table())
                } else {
                    false
                };

                if val.is_table() || is_complex_array {
                    complexes.push((key, val));
                } else {
                    leaves.push((key, val));
                }
            }

            for (key, val) in leaves {
                let child_node = children.get(key.as_str());
                let field_doc = match child_node {
                    Some(DocNode::Leaf(d)) | Some(DocNode::Node(d, _)) => *d,
                    _ => "",
                };

                if !field_doc.is_empty() {
                    output.push_str(&format!("# {}\n", field_doc));
                }

                let val_str = toml::to_string(val).unwrap_or_else(|_| val.to_string());
                output.push_str(&format!("{} = {}\n", escape_toml_key(key), val_str.trim()));
            }

            for (key, val) in complexes {
                let child_node = children.get(key.as_str());
                let mut next_path = path.clone();
                let escaped_key = escape_toml_key(key);
                next_path.push(escaped_key.clone());
                let full_path = next_path.join(".");

                let field_doc = match child_node {
                    Some(DocNode::Node(d, _)) | Some(DocNode::Leaf(d)) => *d,
                    _ => "",
                };

                match val {
                    Value::Array(arr) => {
                        if arr.is_empty() {
                            if !field_doc.is_empty() {
                                output.push_str(&format!("# {}\n", field_doc));
                            }
                            output.push_str(&format!("{} = []\n", escaped_key));
                        } else {
                            for (i, item) in arr.iter().enumerate() {
                                output.push_str("\n");
                                if i == 0 && !field_doc.is_empty() {
                                    output.push_str(&format!("# {}\n", field_doc));
                                }
                                output.push_str(&format!("[[{}]]\n", full_path));
                                if let Some(node) = child_node {
                                    output.push_str(&build_toml_with_comments(
                                        node,
                                        item,
                                        next_path.clone(),
                                    ));
                                }
                            }
                        }
                    }
                    Value::Table(inner_table) => {
                        output.push_str("\n");
                        if !field_doc.is_empty() {
                            output.push_str(&format!("# {}\n", field_doc));
                        }
                        output.push_str(&format!("[{}]\n", full_path));

                        if inner_table.is_empty() {
                        } else if let Some(node) = child_node {
                            output.push_str(&build_toml_with_comments(node, val, next_path));
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }

    output
}

fn escape_toml_key(key: &str) -> String {
    let is_bare_key = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

    if is_bare_key {
        key.to_string()
    } else {
        format!("\"{}\"", key.replace('"', "\\\""))
    }
}

pub fn generate_toml_with_comments<T>() -> String
where
    T: Default + Serialize + DocReader,
{
    let default_instance = T::default();
    let root_node = T::get_recursive_docs();

    let toml_value = Value::try_from(&default_instance).expect("Failed to convert to TOML value");
    build_toml_with_comments(&root_node, &toml_value, Vec::new())
}
