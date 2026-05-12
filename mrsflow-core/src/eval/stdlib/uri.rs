//! `Uri.*` stdlib bindings.
//!
//! v1 uses a hand-rolled URI parser/encoder rather than a dependency. RFC
//! 3986 unreserved set drives percent-encoding; `Uri.Parts` accepts the
//! common form `scheme://[user[:pass]@]host[:port][/path][?query][#fragment]`.

#![allow(unused_imports)]

use crate::parser::Param;

use super::super::env::EnvNode;
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Record, Value};
use super::common::{expect_text, one, two, type_mismatch};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Uri.EscapeDataString", one("text"), escape_data_string),
        ("Uri.Combine", two("baseUri", "relativeUri"), combine),
        ("Uri.BuildQueryString", one("record"), build_query_string),
        ("Uri.Parts", one("uri"), parts),
    ]
}

// --- EscapeDataString ---

fn escape_data_string(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let s = expect_text(&args[0])?;
    Ok(Value::Text(percent_encode(s)))
}

/// Percent-encode every byte that is not in the RFC 3986 unreserved set.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if is_unreserved(b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn is_unreserved(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~')
}

/// Percent-decode `s` into a string. Invalid sequences pass through unchanged.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

// --- Combine ---

fn combine(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let base = expect_text(&args[0])?;
    let rel = expect_text(&args[1])?;
    Ok(Value::Text(combine_str(base, rel)))
}

fn combine_str(base: &str, rel: &str) -> String {
    // Absolute URI → return as-is.
    if has_scheme(rel) {
        return rel.to_string();
    }
    // Scheme-relative ("//host/...") — pick base scheme.
    if let Some(rest) = rel.strip_prefix("//") {
        if let Some(idx) = base.find(':') {
            return format!("{}://{}", &base[..idx], rest);
        }
        return rel.to_string();
    }
    if rel.starts_with('/') {
        // Path-absolute: replace base path with rel.
        if let Some(host_end) = scheme_authority_end(base) {
            return format!("{}{}", &base[..host_end], rel);
        }
        return rel.to_string();
    }
    // Relative path: strip last segment of base, append.
    if base.ends_with('/') {
        format!("{base}{rel}")
    } else {
        let cut = base.rfind('/').map(|i| i + 1).unwrap_or(0);
        format!("{}{}", &base[..cut], rel)
    }
}

fn has_scheme(s: &str) -> bool {
    if let Some(idx) = s.find(':') {
        let scheme = &s[..idx];
        !scheme.is_empty() && scheme.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
    } else {
        false
    }
}

fn scheme_authority_end(s: &str) -> Option<usize> {
    let scheme_end = s.find("://")? + 3;
    let rest = &s[scheme_end..];
    let path_start = rest.find('/').unwrap_or(rest.len());
    Some(scheme_end + path_start)
}

// --- BuildQueryString ---

fn build_query_string(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    if record.fields.is_empty() {
        return Ok(Value::Text(String::new()));
    }
    let mut out = String::from("?");
    for (i, (k, v)) in record.fields.iter().enumerate() {
        if i > 0 {
            out.push('&');
        }
        let forced = super::super::force(v.clone(), &mut |e, env| {
            super::super::evaluate(e, env, &super::super::NoIoHost)
        })?;
        let v_str = value_to_query_text(&forced)?;
        out.push_str(&percent_encode(k));
        out.push('=');
        out.push_str(&percent_encode(&v_str));
    }
    Ok(Value::Text(out))
}

fn value_to_query_text(v: &Value) -> Result<String, MError> {
    match v {
        Value::Text(s) => Ok(s.clone()),
        Value::Number(n) => Ok(format!("{n:?}").trim_end_matches(".0").to_string()),
        Value::Logical(b) => Ok((if *b { "true" } else { "false" }).to_string()),
        Value::Null => Ok(String::new()),
        other => Err(type_mismatch("text/number/logical/null (query value)", other)),
    }
}

// --- Parts ---

fn parts(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let s = expect_text(&args[0])?;
    let (scheme, rest) = match s.find("://") {
        Some(idx) => (s[..idx].to_string(), &s[idx + 3..]),
        None => (String::new(), s),
    };

    // Fragment first (last '#'), then query, then authority+path.
    let (rest, fragment) = match rest.rfind('#') {
        Some(idx) => (&rest[..idx], rest[idx + 1..].to_string()),
        None => (rest, String::new()),
    };
    let (rest, query) = match rest.find('?') {
        Some(idx) => (&rest[..idx], rest[idx + 1..].to_string()),
        None => (rest, String::new()),
    };
    let (authority, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], rest[idx..].to_string()),
        None => (rest, String::new()),
    };

    // Authority = [user[:pass]@]host[:port]
    let (userinfo, hostport) = match authority.rfind('@') {
        Some(idx) => (Some(&authority[..idx]), &authority[idx + 1..]),
        None => (None, authority),
    };
    let (username, password) = match userinfo {
        Some(ui) => match ui.find(':') {
            Some(idx) => (
                percent_decode(&ui[..idx]),
                Some(percent_decode(&ui[idx + 1..])),
            ),
            None => (percent_decode(ui), None),
        },
        None => (String::new(), None),
    };
    let (host, port_str) = match hostport.rfind(':') {
        Some(idx) => (&hostport[..idx], Some(&hostport[idx + 1..])),
        None => (hostport, None),
    };
    let port_v = match port_str {
        Some(p) if !p.is_empty() => match p.parse::<u32>() {
            Ok(n) => Value::Number(n as f64),
            Err(_) => Value::Null,
        },
        _ => Value::Null,
    };

    // Query: parse k=v&k=v pairs into a Record.
    let mut query_fields: Vec<(String, Value)> = Vec::new();
    if !query.is_empty() {
        for pair in query.split('&') {
            let (k, v) = match pair.find('=') {
                Some(idx) => (
                    percent_decode(&pair[..idx]),
                    percent_decode(&pair[idx + 1..]),
                ),
                None => (percent_decode(pair), String::new()),
            };
            query_fields.push((k, Value::Text(v)));
        }
    }
    let query_record = Value::Record(Record {
        fields: query_fields,
        env: EnvNode::empty(),
    });

    let fields = vec![
        ("Scheme".to_string(), Value::Text(scheme)),
        ("Host".to_string(), Value::Text(host.to_string())),
        ("Port".to_string(), port_v),
        ("Path".to_string(), Value::Text(path)),
        ("Query".to_string(), query_record),
        ("Fragment".to_string(), Value::Text(fragment)),
        ("UserName".to_string(), Value::Text(username)),
        (
            "Password".to_string(),
            match password {
                Some(p) => Value::Text(p),
                None => Value::Null,
            },
        ),
    ];
    Ok(Value::Record(Record {
        fields,
        env: EnvNode::empty(),
    }))
}
