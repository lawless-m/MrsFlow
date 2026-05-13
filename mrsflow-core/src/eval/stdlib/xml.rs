//! `Xml.Document` and `Xml.Tables` — XML → navigation table.
//!
//! MS's `Xml.Document` returns a navtable whose columns are
//! `(Name, Namespace, Value, Attributes)`. For an element with children
//! the `Value` cell is itself a recursive `Xml.Document`-shaped table;
//! for a leaf the `Value` is the element's text content. `Attributes`
//! is a record of `name = text` pairs (empty record when the element
//! has no attributes).
//!
//! MS's `Xml.Tables` is documented as the same shape but with smart
//! detection of table-like sub-structures (multiple identically-named
//! children with consistent schemas get flattened). v1 here calls
//! through to `document` and surfaces the same recursive shape — the
//! corpus uses `Xml.Document`; if `Xml.Tables` ever gets corpus use,
//! sharpen this then.

#![allow(unused_imports)]

use crate::parser::Param;

use super::super::env::EnvNode;
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Record, Table, Value};
use super::common::type_mismatch;

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        (
            "Xml.Document",
            vec![
                Param { name: "contents".into(), optional: false, type_annotation: None },
                Param { name: "encoding".into(), optional: true,  type_annotation: None },
            ],
            document,
        ),
        (
            "Xml.Tables",
            vec![
                Param { name: "contents".into(), optional: false, type_annotation: None },
                Param { name: "encoding".into(), optional: true,  type_annotation: None },
            ],
            tables,
        ),
    ]
}

fn document(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let bytes = match &args[0] {
        Value::Binary(b) => b.clone(),
        Value::Text(s) => s.as_bytes().to_vec(),
        other => return Err(type_mismatch("binary or text", other)),
    };
    let root = parse_xml(&bytes)?;
    Ok(Value::Table(nodes_to_table(vec![root])))
}

fn tables(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    // v1: identical to Xml.Document. See module doc.
    document(args, host)
}

/// In-memory representation of one XML element, built by walking the
/// quick-xml event stream. Comments and processing instructions are
/// dropped during the walk; whitespace-only text between elements is
/// dropped too.
struct XmlNode {
    name: String,
    namespace: Option<String>,
    attributes: Vec<(String, String)>,
    content: XmlContent,
}

enum XmlContent {
    /// Element with no children — its text body (possibly empty).
    Text(String),
    /// Element whose body contains other elements (mixed content is
    /// kept structurally — text segments between children are dropped).
    Children(Vec<XmlNode>),
}

fn parse_xml(bytes: &[u8]) -> Result<XmlNode, MError> {
    use quick_xml::events::Event;
    use quick_xml::name::ResolveResult;
    use quick_xml::NsReader;

    let mut reader = NsReader::from_reader(bytes);
    reader.config_mut().trim_text(false);

    // Stack of in-progress parents. Each entry is (node-being-built,
    // accumulated text for this element). The text is used only at
    // element-close: if no children were appended we surface it as
    // XmlContent::Text; if children were appended we drop it (mixed-
    // content text segments aren't represented in MS's navtable).
    let mut stack: Vec<(XmlNode, String)> = Vec::new();
    let mut root: Option<XmlNode> = None;
    let mut buf = Vec::new();

    loop {
        match reader.read_resolved_event_into(&mut buf) {
            Err(e) => {
                return Err(MError::Other(format!(
                    "Xml.Document: parse error at byte {}: {e}",
                    reader.buffer_position()
                )));
            }
            Ok((ns, Event::Start(e))) => {
                let local = std::str::from_utf8(e.local_name().as_ref())
                    .map_err(|_| MError::Other("Xml.Document: non-UTF-8 element name".into()))?
                    .to_string();
                let namespace = match ns {
                    ResolveResult::Bound(b) => Some(
                        std::str::from_utf8(b.as_ref())
                            .map_err(|_| MError::Other("Xml.Document: non-UTF-8 namespace URI".into()))?
                            .to_string(),
                    ),
                    _ => None,
                };
                let attributes = collect_attributes(&e)?;
                stack.push((
                    XmlNode {
                        name: local,
                        namespace,
                        attributes,
                        content: XmlContent::Children(Vec::new()),
                    },
                    String::new(),
                ));
            }
            Ok((ns, Event::Empty(e))) => {
                let local = std::str::from_utf8(e.local_name().as_ref())
                    .map_err(|_| MError::Other("Xml.Document: non-UTF-8 element name".into()))?
                    .to_string();
                let namespace = match ns {
                    ResolveResult::Bound(b) => Some(
                        std::str::from_utf8(b.as_ref())
                            .map_err(|_| MError::Other("Xml.Document: non-UTF-8 namespace URI".into()))?
                            .to_string(),
                    ),
                    _ => None,
                };
                let attributes = collect_attributes(&e)?;
                let node = XmlNode {
                    name: local,
                    namespace,
                    attributes,
                    content: XmlContent::Text(String::new()),
                };
                append_child_or_set_root(&mut stack, &mut root, node);
            }
            Ok((_, Event::End(_))) => {
                let (mut node, text) = stack
                    .pop()
                    .ok_or_else(|| MError::Other("Xml.Document: stray end tag".into()))?;
                node.content = match node.content {
                    XmlContent::Children(cs) if cs.is_empty() => {
                        // No element children — collapse to a text leaf
                        // (after stripping pure whitespace, matching MS's
                        // behaviour for leaf-text-only elements).
                        let t = if text.trim().is_empty() { String::new() } else { text };
                        XmlContent::Text(t)
                    }
                    XmlContent::Children(cs) => XmlContent::Children(cs),
                    other => other,
                };
                append_child_or_set_root(&mut stack, &mut root, node);
            }
            Ok((_, Event::Text(t))) => {
                let raw = t
                    .unescape()
                    .map_err(|e| MError::Other(format!("Xml.Document: text unescape: {e}")))?;
                if let Some((_, txt)) = stack.last_mut() {
                    txt.push_str(&raw);
                }
            }
            Ok((_, Event::CData(c))) => {
                let s = std::str::from_utf8(c.as_ref())
                    .map_err(|_| MError::Other("Xml.Document: non-UTF-8 CDATA".into()))?;
                if let Some((_, txt)) = stack.last_mut() {
                    txt.push_str(s);
                }
            }
            Ok((_, Event::Eof)) => break,
            Ok(_) => {
                // Comments, processing instructions, DOCTYPE, etc — skip.
            }
        }
        buf.clear();
    }

    root.ok_or_else(|| MError::Other("Xml.Document: no root element".into()))
}

fn collect_attributes(
    e: &quick_xml::events::BytesStart,
) -> Result<Vec<(String, String)>, MError> {
    let mut out = Vec::new();
    for attr in e.attributes() {
        let attr = attr
            .map_err(|err| MError::Other(format!("Xml.Document: attribute parse: {err}")))?;
        let key = std::str::from_utf8(attr.key.as_ref())
            .map_err(|_| MError::Other("Xml.Document: non-UTF-8 attribute name".into()))?
            .to_string();
        // Skip xmlns declarations — they shouldn't surface in the
        // Attributes record; the resolved Namespace column covers them.
        if key == "xmlns" || key.starts_with("xmlns:") {
            continue;
        }
        // attr.value is the raw (possibly XML-escaped) byte slice; we
        // unescape it and decode as UTF-8 by hand to avoid needing a
        // Decoder borrow from the reader (its constructor is private).
        let raw = std::str::from_utf8(&attr.value)
            .map_err(|_| MError::Other("Xml.Document: non-UTF-8 attribute value".into()))?;
        let value = quick_xml::escape::unescape(raw)
            .map_err(|err| MError::Other(format!("Xml.Document: attribute unescape: {err}")))?
            .into_owned();
        out.push((key, value));
    }
    Ok(out)
}

fn append_child_or_set_root(
    stack: &mut Vec<(XmlNode, String)>,
    root: &mut Option<XmlNode>,
    node: XmlNode,
) {
    if let Some((parent, _)) = stack.last_mut() {
        if let XmlContent::Children(cs) = &mut parent.content {
            cs.push(node);
        }
    } else {
        *root = Some(node);
    }
}

fn nodes_to_table(nodes: Vec<XmlNode>) -> Table {
    let cols: Vec<String> = vec![
        "Name".to_string(),
        "Namespace".to_string(),
        "Value".to_string(),
        "Attributes".to_string(),
    ];
    let rows: Vec<Vec<Value>> = nodes
        .into_iter()
        .map(|n| {
            let value_cell = match n.content {
                XmlContent::Text(s) => Value::Text(s),
                XmlContent::Children(cs) => Value::Table(nodes_to_table(cs)),
            };
            let ns_cell = match n.namespace {
                Some(s) => Value::Text(s),
                None => Value::Null,
            };
            let attr_cell = Value::Record(Record {
                fields: n
                    .attributes
                    .into_iter()
                    .map(|(k, v)| (k, Value::Text(v)))
                    .collect(),
                env: EnvNode::empty(),
            });
            vec![Value::Text(n.name), ns_cell, value_cell, attr_cell]
        })
        .collect();
    Table::from_rows(cols, rows)
}
