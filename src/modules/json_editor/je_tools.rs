use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub enum ValType {
    Null,
    Bool(bool),
    Number(String),
    Str(String),
    Array(usize),
    Object(usize),
}

impl ValType {
    pub fn preview_str(&self) -> String {
        match self {
            ValType::Null => "null".into(),
            ValType::Bool(b) => if *b { "true" } else { "false" }.into(),
            ValType::Number(n) => n.clone(),
            ValType::Str(s) => format!("\"{}\"", truncate_preview(s, 80)),
            ValType::Array(n) => format!("[{} item{}]", n, if *n == 1 { "" } else { "s" }),
            ValType::Object(n) => format!("{{{} key{}}}", n, if *n == 1 { "" } else { "s" }),
        }
    }

    pub fn type_label(&self) -> &'static str {
        match self {
            ValType::Null => "null",
            ValType::Bool(_) => "bool",
            ValType::Number(_) => "number",
            ValType::Str(_) => "string",
            ValType::Array(_) => "array",
            ValType::Object(_) => "object",
        }
    }

    pub fn is_container(&self) -> bool { matches!(self, ValType::Array(_) | ValType::Object(_)) }
}

fn truncate_preview(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let head: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() { format!("{}...", head) } else { head }
}

#[derive(Debug, Clone)]
pub struct FlatNode {
    pub key: String,
    pub val_type: ValType,
    pub depth: u32,
    pub is_expanded: bool,
    pub path: Vec<String>,
    pub has_children: bool,
    pub is_array_index: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortMode { None, KeyAsc, KeyDesc, ValueAsc, ValueDesc, }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchTarget { Keys, Values, Both, }

pub fn path_key(path: &[String]) -> String { path.join("\x00") }

pub fn flatten_value(value: &Value, key: &str, depth: u32, path: &[String], expanded: &HashSet<String>, sort_mode: SortMode, out: &mut Vec<FlatNode>) {
    let node_key = path_key(path);
    let is_expanded = expanded.contains(&node_key);
    let val_type = value_to_type(value);
    let has_children = val_type.is_container();
    let is_array_index = key.parse::<usize>().is_ok();

    out.push(FlatNode {
        key: key.to_string(),
        val_type: val_type.clone(),
        depth,
        is_expanded,
        path: path.to_vec(),
        has_children,
        is_array_index,
    });

    if !is_expanded || !has_children { return; }

    match value {
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            sort_entries_obj(&mut entries, sort_mode);
            for (k, v) in entries {
                let mut child_path = path.to_vec();
                child_path.push(k.clone());
                flatten_value(v, k, depth + 1, &child_path, expanded, sort_mode, out);
            }
        }
        Value::Array(arr) => {
            let pairs: Vec<(String, &Value)> = arr.iter().enumerate()
                .map(|(i, v)| (i.to_string(), v))
                .collect();
            let mut refs: Vec<(&String, &Value)> = pairs.iter()
                .map(|(k, v)| (k, *v))
                .collect();
            sort_entries_obj(&mut refs, sort_mode);
            for (k, v) in refs {
                let mut child_path = path.to_vec();
                child_path.push(k.clone());
                flatten_value(v, k, depth + 1, &child_path, expanded, sort_mode, out);
            }
        }
        _ => {}
    }
}

pub fn build_flat(root: &Value, scope: &[String], expanded: &HashSet<String>, sort_mode: SortMode, ) -> Vec<FlatNode> {
    let scoped = value_at_path(root, scope).unwrap_or(root);
    let root_key = scope.last().map(|s| s.as_str()).unwrap_or("root");

    let mut out = Vec::new();
    flatten_value(scoped, root_key, 0, scope, expanded, sort_mode, &mut out);
    out
}

fn sort_entries_obj(entries: &mut Vec<(&String, &Value)>, mode: SortMode) {
    match mode {
        SortMode::None => {}
        SortMode::KeyAsc => entries.sort_by(|a, b| a.0.cmp(b.0)),
        SortMode::KeyDesc => entries.sort_by(|a, b| b.0.cmp(a.0)),
        SortMode::ValueAsc => entries.sort_by(|a, b| value_sort_key(a.1).cmp(&value_sort_key(b.1))),
        SortMode::ValueDesc => entries.sort_by(|a, b| value_sort_key(b.1).cmp(&value_sort_key(a.1))),
    }
}

fn value_sort_key(v: &Value) -> String {
    match v {
        Value::Null => "\x00".into(),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                format!("{:020.6}", f)
            } else {
                n.to_string()
            }
        }
        Value::String(s) => s.clone(),
        Value::Array(a) => format!("[array:{}]", a.len()),
        Value::Object(m) => format!("{{object:{}}}", m.len()),
    }
}

fn value_to_type(v: &Value) -> ValType {
    match v {
        Value::Null => ValType::Null,
        Value::Bool(b) => ValType::Bool(*b),
        Value::Number(n) => ValType::Number(n.to_string()),
        Value::String(s) => ValType::Str(s.clone()),
        Value::Array(a) => ValType::Array(a.len()),
        Value::Object(m) => ValType::Object(m.len()),
    }
}

pub fn value_at_path<'a>(root: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut cur = root;
    for seg in path {
        cur = match cur {
            Value::Object(m) => m.get(seg)?,
            Value::Array(a)  => {
                let idx: usize = seg.parse().ok()?;
                a.get(idx)?
            }
            _ => return None,
        };
    }
    Some(cur)
}

pub fn value_at_path_mut<'a>(root: &'a mut Value, path: &[String]) -> Option<&'a mut Value> {
    let mut cur = root;
    for seg in path {
        cur = match cur {
            Value::Object(m) => m.get_mut(seg)?,
            Value::Array(a)  => {
                let idx: usize = seg.parse().ok()?;
                a.get_mut(idx)?
            }
            _ => return None,
        };
    }
    Some(cur)
}

pub fn delete_at_path(root: &mut Value, path: &[String]) -> bool {
    if path.is_empty() { return false; }
    let (parent_path, key) = path.split_at(path.len() - 1);
    let key = &key[0];
    let parent = match value_at_path_mut(root, parent_path) {
        Some(v) => v,
        None => return false,
    };
    match parent {
        Value::Object(m) => { m.remove(key); true }
        Value::Array(a)  => {
            if let Ok(idx) = key.parse::<usize>() {
                if idx < a.len() { a.remove(idx); return true; }
            }
            false
        }
        _ => false,
    }
}

pub fn set_at_path(root: &mut Value, path: &[String], new_val: Value) -> bool {
    if path.is_empty() { *root = new_val; return true; }
    let (parent_path, key) = path.split_at(path.len() - 1);
    let key = &key[0];
    let parent = match value_at_path_mut(root, parent_path) {
        Some(v) => v,
        None => return false,
    };
    match parent {
        Value::Object(m) => { m.insert(key.clone(), new_val); true }
        Value::Array(a)  => {
            if let Ok(idx) = key.parse::<usize>() {
                if idx < a.len() { a[idx] = new_val; return true; }
            }
            false
        }
        _ => false,
    }
}

pub fn add_key_at_path(root: &mut Value, parent_path: &[String], key: &str, new_val: Value) -> bool {
    let parent = match value_at_path_mut(root, parent_path) {
        Some(v) => v,
        None => return false,
    };
    match parent {
        Value::Object(m) => { m.insert(key.to_string(), new_val); true }
        Value::Array(a)  => { a.push(new_val); true }
        _ => false,
    }
}

pub fn rename_key_at_path(root: &mut Value, parent_path: &[String], old_key: &str, new_key: &str,) -> bool {
    let parent = match value_at_path_mut(root, parent_path) {
        Some(v) => v,
        None => return false,
    };
    match parent {
        Value::Object(m) => {
            if let Some(val) = m.remove(old_key) {
                m.insert(new_key.to_string(), val);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

pub fn search_flat(nodes: &[FlatNode], query: &str, target: SearchTarget,) -> Vec<usize> {
    if query.is_empty() { return Vec::new(); }
    let q = query.to_lowercase();
    let mut results = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        let key_match = node.key.to_lowercase().contains(&q);
        let val_str = match &node.val_type {
            ValType::Null => "null".to_string(),
            ValType::Bool(b) => if *b { "true".to_string() } else { "false".to_string() },
            ValType::Number(n) => n.clone(),
            ValType::Str(s) => s.to_lowercase(),
            ValType::Array(_) => String::new(),
            ValType::Object(_) => String::new(),
        };
        let val_match = val_str.contains(&q);

        let hit = match target {
            SearchTarget::Keys => key_match,
            SearchTarget::Values => val_match,
            SearchTarget::Both => key_match || val_match,
        };
        if hit {
            results.push(i);
        }
    }
    results
}

fn search_recursive(value: &Value, path: &[String], q: &str, target: SearchTarget, out: &mut Vec<Vec<String>>) {
    let key = path.last().map(|s| s.as_str()).unwrap_or("");
    let key_match = key.to_lowercase().contains(q);
    let val_match = match value {
        Value::Null => "null".contains(q),
        Value::Bool(true) => "true".contains(q),
        Value::Bool(false) => "false".contains(q),
        Value::Number(n) => n.to_string().to_lowercase().contains(q),
        Value::String(s) => s.to_lowercase().contains(q),
        _ => false,
    };
    let hit = match target {
        SearchTarget::Keys => key_match,
        SearchTarget::Values => val_match,
        SearchTarget::Both => key_match || val_match,
    };
    if hit && !path.is_empty() { out.push(path.to_vec()); }
    match value {
        Value::Object(m) => m.iter().for_each(|(k, v)| {
            let mut cp = path.to_vec(); cp.push(k.clone());
            search_recursive(v, &cp, q, target, out);
        }),
        Value::Array(a) => a.iter().enumerate().for_each(|(i, v)| {
            let mut cp = path.to_vec(); cp.push(i.to_string());
            search_recursive(v, &cp, q, target, out);
        }),
        _ => {}
    }
}

pub fn search_all_nodes(root: &Value, scope: &[String], query: &str, target: SearchTarget) -> Vec<Vec<String>> {
    if query.is_empty() { return Vec::new(); }
    let q = query.to_lowercase();
    let scoped = value_at_path(root, scope).unwrap_or(root);
    let mut out = Vec::new();
    search_recursive(scoped, scope, &q, target, &mut out);
    out
}

pub fn serialize_value(value: &Value, pretty: bool) -> String {
    if pretty { serde_json::to_string_pretty(value).unwrap_or_default() } 
    else { serde_json::to_string(value).unwrap_or_default() }
}

pub fn parse_text(text: &str) -> Result<Value, (String, usize)> {
    serde_json::from_str(text).map_err(|e| {
        let line = e.line();
        (e.to_string(), line)
    })
}

pub fn validate_json(text: &str) -> Vec<(usize, String)> {
    if text.trim().is_empty() { return Vec::new(); }
    match serde_json::from_str::<Value>(text) {
        Ok(_) => Vec::new(),
        Err(e) => vec![(e.line(), e.to_string())],
    }
}

pub fn parse_cell_value(raw: &str) -> Value {
    let t = raw.trim();
    match t {
        "null" => Value::Null,
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => {
            if let Ok(n) = serde_json::from_str::<serde_json::Number>(t) { return Value::Number(n); }
            if let Ok(v) = serde_json::from_str::<Value>(t) { return v; }
            Value::String(t.to_string())
        }
    }
}

pub fn parse_edit_value(raw: &str) -> Value {
    let t = raw.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        return Value::String(t[1..t.len() - 1].to_string());
    }
    parse_cell_value(t)
}

pub fn expand_recursive(root: &Value, scope: &[String], path: &[String], max_depth: u32, expanded: &mut HashSet<String>,) {
    let val = match value_at_path(root, path) {
        Some(v) => v,
        None => return,
    };
    let depth = path.len().saturating_sub(scope.len()) as u32;
    if depth > max_depth { return; }

    if val.is_object() || val.is_array() {
        expanded.insert(path_key(path));
        let keys: Vec<String> = match val {
            Value::Object(m) => m.keys().cloned().collect(),
            Value::Array(a)  => (0..a.len()).map(|i| i.to_string()).collect(),
            _ => vec![],
        };
        for k in keys {
            let mut child = path.to_vec();
            child.push(k);
            expand_recursive(root, scope, &child, max_depth, expanded);
        }
    }
}

pub fn collapse_recursive(path: &[String], expanded: &mut HashSet<String>) {
    let key = path_key(path);
    expanded.remove(&key);
    let prefix = format!("{}\x00", key);
    expanded.retain(|k| !k.starts_with(&prefix) && k != &key);
}

pub fn sort_label(m: SortMode) -> &'static str {
    match m {
        SortMode::None => "None",
        SortMode::KeyAsc => "Key A-Z",
        SortMode::KeyDesc => "Key Z-A",
        SortMode::ValueAsc => "Value A-Z",
        SortMode::ValueDesc => "Value Z-A",
    }
}

pub fn search_target_label(t: SearchTarget) -> &'static str {
    match t {
        SearchTarget::Both => "Both",
        SearchTarget::Keys => "Keys",
        SearchTarget::Values => "Values",
    }
}
