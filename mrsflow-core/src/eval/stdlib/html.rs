//! `Html.Table` — CSS-selector-driven HTML → Table extraction.
//!
//! Sharper sibling of `Web.Page` (which tries to auto-detect tables):
//! you tell `Html.Table` exactly which CSS selector defines each
//! column and, optionally, which selects a row. Each column spec is
//! a 2- or 3-element list:
//!   `{ColumnName, "css-selector"}`                     — take inner text
//!   `{ColumnName, "css-selector", each TransformFn(_)}`— pass the matched
//!                                                       element's record
//!                                                       to the closure
//!
//! The record passed to the transform contains `Name` (tag), `Value`
//! (inner text), and `Attributes` (record of `attr = text`). That's
//! the shape MS's docs imply via `each [Attributes][href]`.

#![allow(unused_imports)]

use crate::parser::Param;

use scraper::{ElementRef, Html, Selector};

use super::super::env::EnvNode;
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Record, Table, Value};
use super::common::{expect_text, invoke_callback_with_host, type_mismatch};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![(
        "Html.Table",
        vec![
            Param { name: "html".into(), optional: false, type_annotation: None },
            Param { name: "columnNameSelectorPairs".into(), optional: false, type_annotation: None },
            Param { name: "options".into(), optional: true,  type_annotation: None },
        ],
        table,
    )]
}

struct ColumnSpec {
    name: String,
    selector: Selector,
    transform: Option<crate::eval::value::Closure>,
}

fn table(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let html_src = match &args[0] {
        Value::Text(s) => s.clone(),
        Value::Binary(b) => String::from_utf8(b.clone())
            .map_err(|_| MError::Other("Html.Table: html bytes are not valid UTF-8".into()))?,
        other => return Err(type_mismatch("text or binary", other)),
    };

    // Deep-force the column specs and options record so nested
    // lazy fields (text selectors, RowSelector, transform closures)
    // are resolved before pattern-matching. The transform closures
    // themselves stay as Value::Function — those are forced already
    // since closure construction is eager.
    let specs_forced = super::super::deep_force(args[1].clone(), host)?;
    let opts_forced: Option<Value> = match args.get(2) {
        None | Some(Value::Null) => None,
        Some(v) => Some(super::super::deep_force(v.clone(), host)?),
    };

    let specs = parse_column_specs(&specs_forced)?;
    let row_selector = parse_row_selector(opts_forced.as_ref())?;

    let doc = Html::parse_document(&html_src);
    let root_ref = doc.root_element();

    // PQ's algorithm: each column selector is applied to the whole
    // document; row N's cell is the Nth document-order match. The
    // RowSelector controls the row count (= number of matches it
    // produces). Without RowSelector, the table has one row (each
    // column's first match). This matches both MS doc examples.
    let row_count = match &row_selector {
        Some(sel) => root_ref.select(sel).count(),
        None => 1,
    };

    // Precompute document-order matches for each column.
    let column_matches: Vec<Vec<ElementRef>> = specs
        .iter()
        .map(|s| root_ref.select(&s.selector).collect())
        .collect();

    let col_names: Vec<String> = specs.iter().map(|s| s.name.clone()).collect();
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(row_count);
    for row_idx in 0..row_count {
        let mut cells: Vec<Value> = Vec::with_capacity(specs.len());
        for (col_idx, spec) in specs.iter().enumerate() {
            let matched = column_matches[col_idx].get(row_idx).copied();
            let cell = match (matched, &spec.transform) {
                (None, _) => Value::Null,
                (Some(el), Some(closure)) => {
                    let record = element_to_record(el);
                    invoke_callback_with_host(closure, vec![record], host)?
                }
                (Some(el), None) => Value::Text(inner_text(el)),
            };
            cells.push(cell);
        }
        rows.push(cells);
    }
    Ok(Value::Table(Table::from_rows(col_names, rows)))
}

fn parse_column_specs(v: &Value) -> Result<Vec<ColumnSpec>, MError> {
    let outer = match v {
        Value::List(xs) => xs,
        other => return Err(type_mismatch("list of column specs", other)),
    };
    let mut out: Vec<ColumnSpec> = Vec::with_capacity(outer.len());
    for entry in outer.iter() {
        let parts: &Vec<Value> = match entry {
            Value::List(xs) => xs.as_ref(),
            other => return Err(type_mismatch("list (column spec)", other)),
        };
        if parts.len() < 2 || parts.len() > 3 {
            return Err(MError::Other(format!(
                "Html.Table: column spec must have 2 or 3 elements, got {}",
                parts.len()
            )));
        }
        let name = match &parts[0] {
            Value::Text(s) => s.clone(),
            other => return Err(type_mismatch("text (column name)", other)),
        };
        let sel_src = match &parts[1] {
            Value::Text(s) => s.clone(),
            other => return Err(type_mismatch("text (css selector)", other)),
        };
        let selector = Selector::parse(&sel_src).map_err(|e| {
            MError::Other(format!("Html.Table: invalid CSS selector {sel_src:?}: {e}"))
        })?;
        let transform = if parts.len() == 3 {
            match &parts[2] {
                Value::Function(c) => Some(c.clone()),
                Value::Null => None,
                other => return Err(type_mismatch("function or null (transform)", other)),
            }
        } else {
            None
        };
        out.push(ColumnSpec { name, selector, transform });
    }
    Ok(out)
}

fn parse_row_selector(v: Option<&Value>) -> Result<Option<Selector>, MError> {
    let rec = match v {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::Record(r)) => r,
        Some(other) => return Err(type_mismatch("record (options)", other)),
    };
    for (k, v) in &rec.fields {
        if k == "RowSelector" {
            let sel_src = match v {
                Value::Text(s) => s.clone(),
                Value::Null => return Ok(None),
                other => return Err(type_mismatch("text (RowSelector)", other)),
            };
            let sel = Selector::parse(&sel_src).map_err(|e| {
                MError::Other(format!("Html.Table: invalid RowSelector {sel_src:?}: {e}"))
            })?;
            return Ok(Some(sel));
        }
    }
    Ok(None)
}

fn inner_text(el: ElementRef) -> String {
    // Concatenate descendant text nodes, like the browser's textContent.
    el.text().collect::<Vec<_>>().concat()
}

fn element_to_record(el: ElementRef) -> Value {
    let name = el.value().name().to_string();
    let value = inner_text(el);
    let attrs: Vec<(String, Value)> = el
        .value()
        .attrs()
        .map(|(k, v)| (k.to_string(), Value::Text(v.to_string())))
        .collect();
    Value::Record(Record {
        fields: vec![
            ("Name".to_string(), Value::Text(name)),
            ("Value".to_string(), Value::Text(value)),
            (
                "Attributes".to_string(),
                Value::Record(Record { fields: attrs, env: EnvNode::empty() }),
            ),
        ],
        env: EnvNode::empty(),
    })
}
