//! `Web.Contents` — HTTP GET, returns response body as binary.
//!
//! Supported options-record fields:
//! - `Headers` — record of `name = text` pairs forwarded as request headers
//! - `ManualStatusHandling` — list of numeric status codes the caller
//!   wants to handle itself (body returned instead of erroring)
//! - `Content` — binary or text body; when present switches the request
//!   to POST. Caller sets `Content-Type` via `Headers` if needed.
//!
//! Other PQ fields (`Query`, `RelativePath`, `Timeout`, `ApiKeyName`,
//! `IsRetry`, `ManualCredentials`) are silently ignored — PQ accepts
//! extra fields too, so this matches its leniency.

use crate::parser::Param;

use super::super::env::EnvNode;
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Record, Value};
use super::common::{expect_text, type_mismatch};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        (
            "Web.Contents",
            vec![
                Param { name: "url".into(),     optional: false, type_annotation: None },
                Param { name: "options".into(), optional: true,  type_annotation: None },
            ],
            contents,
        ),
        (
            "Web.Headers",
            vec![
                Param { name: "url".into(),     optional: false, type_annotation: None },
                Param { name: "options".into(), optional: true,  type_annotation: None },
            ],
            headers,
        ),
    ]
}

fn headers(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let url = expect_text(&args[0])?;
    // Reuse the Web.Contents options parser, but only the Headers field
    // matters for a HEAD request — Content / ManualStatusHandling are
    // ignored here.
    let opts = match args.get(1) {
        None | Some(Value::Null) => Options::default(),
        Some(Value::Record(r)) => parse_options(r, host)?,
        Some(other) => return Err(type_mismatch("record or null", other)),
    };
    let pairs = host
        .web_headers(url, &opts.headers)
        .map_err(|e| MError::Other(format!("Web.Headers({url:?}): {e:?}")))?;
    // Returns a record of header-name = value, matching PQ's Web.Headers.
    let fields: Vec<(String, Value)> = pairs
        .into_iter()
        .map(|(k, v)| (k, Value::Text(v)))
        .collect();
    Ok(Value::Record(Record { fields, env: EnvNode::empty() }))
}

fn contents(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let url = expect_text(&args[0])?;
    let opts = match args.get(1) {
        None | Some(Value::Null) => Options::default(),
        Some(Value::Record(r)) => parse_options(r, host)?,
        Some(other) => return Err(type_mismatch("record or null", other)),
    };
    let content_ref: Option<&[u8]> = opts.content.as_deref();
    let bytes = host
        .web_contents(url, &opts.headers, &opts.manual_status, content_ref)
        .map_err(|e| MError::Other(format!("Web.Contents({url:?}): {e:?}")))?;
    Ok(Value::Binary(bytes))
}

#[derive(Default)]
struct Options {
    headers: Vec<(String, String)>,
    manual_status: Vec<u16>,
    content: Option<Vec<u8>>,
}

fn parse_options(r: &Record, host: &dyn IoHost) -> Result<Options, MError> {
    let mut out = Options::default();
    for (k, v) in &r.fields {
        // Record fields are lazy; force before matching on type.
        let v = force_value(v.clone(), host)?;
        match k.as_str() {
            "Headers" => match v {
                Value::Record(h) => {
                    for (hn, hv) in &h.fields {
                        let hv = force_value(hv.clone(), host)?;
                        let s = match hv {
                            Value::Text(s) => s,
                            // Empty Authorization in the corpus is a PQ
                            // sentinel for "use ambient credentials". We
                            // pass it through as an empty string — the
                            // request just won't carry that header.
                            Value::Null => String::new(),
                            other => return Err(type_mismatch("text", &other)),
                        };
                        out.headers.push((hn.clone(), s));
                    }
                }
                Value::Null => {}
                other => return Err(type_mismatch("record", &other)),
            },
            "ManualStatusHandling" => match v {
                Value::List(xs) => {
                    for x in xs {
                        let x = force_value(x, host)?;
                        match x {
                            Value::Number(n) => {
                                if !n.is_finite() || n < 0.0 || n > 599.0 {
                                    return Err(MError::Other(format!(
                                        "Web.Contents: ManualStatusHandling code out of range: {n}"
                                    )));
                                }
                                out.manual_status.push(n as u16);
                            }
                            other => return Err(type_mismatch("number", &other)),
                        }
                    }
                }
                Value::Null => {}
                other => return Err(type_mismatch("list", &other)),
            },
            "Content" => match v {
                Value::Binary(b) => out.content = Some(b),
                // Text body: encode as UTF-8 bytes. Matches PQ's tolerance
                // for `Content = "foo"` (it lifts to binary internally).
                Value::Text(s) => out.content = Some(s.into_bytes()),
                Value::Null => {}
                other => return Err(type_mismatch("binary or text", &other)),
            },
            _ => {} // ignored fields — see module doc
        }
    }
    Ok(out)
}

fn force_value(v: Value, host: &dyn IoHost) -> Result<Value, MError> {
    super::super::force(v, &mut |e, env| super::super::evaluate(e, env, host))
}
