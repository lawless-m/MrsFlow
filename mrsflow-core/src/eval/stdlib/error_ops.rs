//! `Error.*` stdlib bindings. Just one entry — the canonical
//! constructor for the record used with the `error` keyword.

#![allow(unused_imports)]

use crate::parser::Param;

use super::super::env::EnvNode;
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Record, Value};
use super::common::{expect_text, three, type_mismatch};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![("Error.Record", three("reason", "message", "detail"), error_record)]
}

fn error_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let reason = expect_text(&args[0])?.to_string();
    let message = expect_text(&args[1])?.to_string();
    Ok(Value::Record(Record {
        fields: vec![
            ("Reason".to_string(), Value::Text(reason)),
            ("Message".to_string(), Value::Text(message)),
            ("Detail".to_string(), args[2].clone()),
        ],
        env: EnvNode::empty(),
    }))
}
