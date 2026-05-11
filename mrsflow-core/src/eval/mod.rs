//! Evaluator for the M language.
//!
//! Pure and synchronous; all IO routes through the `IoHost` trait that the
//! shell provides. See `mrsflow/07-evaluator-design.md` for the load-bearing
//! decisions (laziness, error model, environment, value representation).
//!
//! This module is currently a scaffold — the actual `evaluate` logic lands
//! in slice-1 work (tasks #33 and #34). The shape of `Value`, `Env`, and
//! `IoHost` is in place so subsequent slices can fill behaviour without
//! disrupting downstream consumers.

mod env;
mod iohost;
mod value;

pub use env::{Env, EnvNode, EnvOps};
pub use iohost::{IoError, IoHost, NoIoHost};
pub use value::{Closure, MError, Record, Table, ThunkState, TypeRep, Value};

use std::rc::Rc;

use crate::parser::{BinaryOp, Expr, ListItem, Param, UnaryOp};

/// Evaluate an M AST against the given environment, using `host` for any
/// IO-doing stdlib calls. Returns the resulting Value or an `MError`.
///
/// Stub: returns `MError::NotImplemented` until slice-1 fills in literal /
/// identifier / operator / `if` / `let` evaluation.
pub fn evaluate(ast: &Expr, env: &Env, host: &dyn IoHost) -> Result<Value, MError> {
    match ast {
        // --- Literals ---
        Expr::NumberLit(s) => Ok(Value::Number(parse_number_literal(s)?)),
        Expr::TextLit(s) => Ok(Value::Text(s.clone())),
        Expr::LogicalLit(b) => Ok(Value::Logical(*b)),
        Expr::NullLit => Ok(Value::Null),

        // --- Identifier reference: lookup, then force the (possibly thunked) value. ---
        Expr::Identifier(name) => {
            let value = env
                .lookup(name)
                .ok_or_else(|| MError::NameNotInScope(name.clone()))?;
            force(value, &mut |e, env| evaluate(e, env, host))
        }

        // --- Unary ---
        Expr::Unary(op, inner) => {
            let v = evaluate(inner, env, host)?;
            apply_unary(*op, v)
        }

        // --- Binary: short-circuit logical first, then everything else ---
        Expr::Binary(BinaryOp::And, l, r) => {
            let lv = evaluate(l, env, host)?;
            match lv {
                Value::Logical(false) => Ok(Value::Logical(false)),
                Value::Logical(true) => {
                    let rv = evaluate(r, env, host)?;
                    match rv {
                        Value::Logical(b) => Ok(Value::Logical(b)),
                        other => Err(MError::TypeMismatch {
                            expected: "logical",
                            found: type_name(&other),
                        }),
                    }
                }
                other => Err(MError::TypeMismatch {
                    expected: "logical",
                    found: type_name(&other),
                }),
            }
        }
        Expr::Binary(BinaryOp::Or, l, r) => {
            let lv = evaluate(l, env, host)?;
            match lv {
                Value::Logical(true) => Ok(Value::Logical(true)),
                Value::Logical(false) => {
                    let rv = evaluate(r, env, host)?;
                    match rv {
                        Value::Logical(b) => Ok(Value::Logical(b)),
                        other => Err(MError::TypeMismatch {
                            expected: "logical",
                            found: type_name(&other),
                        }),
                    }
                }
                other => Err(MError::TypeMismatch {
                    expected: "logical",
                    found: type_name(&other),
                }),
            }
        }
        Expr::Binary(op, l, r) => {
            let lv = evaluate(l, env, host)?;
            let rv = evaluate(r, env, host)?;
            apply_binary(*op, lv, rv)
        }

        // --- Conditional ---
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let c = evaluate(cond, env, host)?;
            match c {
                Value::Logical(true) => evaluate(then_branch, env, host),
                Value::Logical(false) => evaluate(else_branch, env, host),
                other => Err(MError::TypeMismatch {
                    expected: "logical",
                    found: type_name(&other),
                }),
            }
        }

        // --- Let with lazy mutual-recursive bindings ---
        Expr::Let { bindings, body } => {
            let lazy: Vec<(String, Expr)> = bindings
                .iter()
                .map(|(name, expr)| (name.clone(), expr.clone()))
                .collect();
            let new_env = env.extend_lazy(lazy);
            evaluate(body, &new_env, host)
        }

        // --- Function literal: capture current env in a closure. Type
        //     annotations on params and the return-type annotation are
        //     parsed but ignored at runtime (eval-5 enforces). ---
        Expr::Function {
            params,
            return_type: _,
            body,
        } => Ok(Value::Function(Closure {
            params: params.clone(),
            body: body.clone(),
            env: Rc::clone(env),
        })),

        // --- `each E` is sugar for `(_) => E`. Build the closure directly. ---
        Expr::Each(body) => Ok(Value::Function(Closure {
            params: vec![Param {
                name: "_".to_string(),
                optional: false,
                type_annotation: None,
            }],
            body: body.clone(),
            env: Rc::clone(env),
        })),

        // --- Function invocation: eager arg evaluation, env extension, body eval. ---
        Expr::Invoke { target, args } => {
            let target_v = evaluate(target, env, host)?;
            let target_v = force(target_v, &mut |e, env| evaluate(e, env, host))?;
            let closure = match target_v {
                Value::Function(c) => c,
                other => {
                    return Err(MError::TypeMismatch {
                        expected: "function",
                        found: type_name(&other),
                    });
                }
            };

            let required_count = closure.params.iter().filter(|p| !p.optional).count();
            let max_count = closure.params.len();
            if args.len() < required_count || args.len() > max_count {
                return Err(MError::Other(format!(
                    "invoke: arity mismatch: expected {} required (up to {} total), got {}",
                    required_count,
                    max_count,
                    args.len()
                )));
            }

            // Eagerly evaluate each arg, forcing thunks before binding
            // (M is not call-by-name; design doc §1).
            let mut arg_values = Vec::with_capacity(max_count);
            for arg in args.iter() {
                let v = evaluate(arg, env, host)?;
                let v = force(v, &mut |e, env| evaluate(e, env, host))?;
                arg_values.push(v);
            }
            // Optional params with no supplied arg → null per spec.
            while arg_values.len() < max_count {
                arg_values.push(Value::Null);
            }

            // Extend the closure's captured env with the bound params.
            let mut call_env = Rc::clone(&closure.env);
            for (param, value) in closure.params.iter().zip(arg_values.into_iter()) {
                call_env = call_env.extend(param.name.clone(), value);
            }

            evaluate(&closure.body, &call_env, host)
        }

        // --- List literal: items are eagerly evaluated (only records have
        //     per-field laziness per spec). Range items expand to inclusive
        //     integer sequences. ---
        Expr::List(items) => {
            let mut values = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    ListItem::Single(e) => {
                        let v = evaluate(e, env, host)?;
                        let v = force(v, &mut |e, env| evaluate(e, env, host))?;
                        values.push(v);
                    }
                    ListItem::Range(start, end) => {
                        let sv = evaluate(start, env, host)?;
                        let sv = force(sv, &mut |e, env| evaluate(e, env, host))?;
                        let ev = evaluate(end, env, host)?;
                        let ev = force(ev, &mut |e, env| evaluate(e, env, host))?;
                        let s = expect_number(&sv)?;
                        let e = expect_number(&ev)?;
                        if s.fract() != 0.0 || e.fract() != 0.0 {
                            return Err(MError::Other(format!(
                                "range bounds must be integers, got {} and {}",
                                s, e
                            )));
                        }
                        if s > e {
                            return Err(MError::Other(format!(
                                "range start must be <= end, got {}..{}",
                                s, e
                            )));
                        }
                        let mut i = s as i64;
                        let end_i = e as i64;
                        while i <= end_i {
                            values.push(Value::Number(i as f64));
                            i += 1;
                        }
                    }
                }
            }
            Ok(Value::List(values))
        }

        // --- Record literal: each field is a thunk in a shared env so
        //     siblings can reference one another. The record holds the env
        //     strongly so its thunks remain forceable after the record
        //     escapes its construction scope. ---
        Expr::Record(fields) => {
            let lazy: Vec<(String, Expr)> = fields
                .iter()
                .map(|(name, expr)| (name.clone(), expr.clone()))
                .collect();
            let record_env = env.extend_lazy(lazy);
            let record_fields: Vec<(String, Value)> = fields
                .iter()
                .map(|(name, _)| {
                    let v = record_env
                        .lookup(name)
                        .expect("just bound by extend_lazy");
                    (name.clone(), v)
                })
                .collect();
            Ok(Value::Record(Record {
                fields: record_fields,
                env: record_env,
            }))
        }

        // --- Field access: r[name] (required) or r[name]? (optional). ---
        Expr::FieldAccess {
            target,
            field,
            optional,
        } => {
            let t = evaluate(target, env, host)?;
            let t = force(t, &mut |e, env| evaluate(e, env, host))?;
            let record = match t {
                Value::Record(r) => r,
                other => {
                    return Err(MError::TypeMismatch {
                        expected: "record",
                        found: type_name(&other),
                    });
                }
            };
            match record.fields.iter().find(|(n, _)| n == field) {
                Some((_, v)) => force(v.clone(), &mut |e, env| evaluate(e, env, host)),
                None => {
                    if *optional {
                        Ok(Value::Null)
                    } else {
                        Err(MError::Other(format!("field not found: {}", field)))
                    }
                }
            }
        }

        // --- Item access: list-only for slice 3. r{i} (required) or r{i}? (optional). ---
        Expr::ItemAccess {
            target,
            index,
            optional,
        } => {
            let t = evaluate(target, env, host)?;
            let t = force(t, &mut |e, env| evaluate(e, env, host))?;
            let idx_v = evaluate(index, env, host)?;
            let idx_v = force(idx_v, &mut |e, env| evaluate(e, env, host))?;
            match t {
                Value::List(items) => {
                    let i = expect_number(&idx_v)?;
                    if i.fract() != 0.0 || i < 0.0 {
                        return Err(MError::Other(format!(
                            "list index must be non-negative integer, got {}",
                            i
                        )));
                    }
                    let idx = i as usize;
                    if idx >= items.len() {
                        if *optional {
                            Ok(Value::Null)
                        } else {
                            Err(MError::Other(format!(
                                "list index out of bounds: {} (len {})",
                                idx,
                                items.len()
                            )))
                        }
                    } else {
                        force(items[idx].clone(), &mut |e, env| evaluate(e, env, host))
                    }
                }
                Value::Record(_) | Value::Table(_) => Err(MError::NotImplemented(
                    "record/table item access deferred to a later slice",
                )),
                other => Err(MError::TypeMismatch {
                    expected: "list",
                    found: type_name(&other),
                }),
            }
        }

        // --- `error <expr>` — build an error record (or use a record
        //     operand directly) and raise it. Text operands are lifted to
        //     the standard [Reason, Message, Detail] shape per spec. ---
        Expr::Error(inner) => {
            let v = evaluate(inner, env, host)?;
            let v = force(v, &mut |e, env| evaluate(e, env, host))?;
            let record = match v {
                Value::Text(msg) => build_standard_error_record(
                    "Expression.Error".to_string(),
                    msg,
                ),
                Value::Record(r) => Value::Record(r),
                other => {
                    return Err(MError::TypeMismatch {
                        expected: "text or record",
                        found: type_name(&other),
                    });
                }
            };
            Err(MError::Raised(record))
        }

        // --- `try body` / `try body otherwise fallback` ---
        Expr::Try { body, otherwise } => {
            let result = evaluate(body, env, host);
            match (result, otherwise) {
                // No otherwise: success → wrap with HasError=false, Value=v.
                (Ok(v), None) => {
                    let v = force(v, &mut |e, env| evaluate(e, env, host))?;
                    Ok(try_success_record(v))
                }
                // No otherwise: failure → wrap with HasError=true, Error=<rec>.
                (Err(err), None) => Ok(try_failure_record(error_to_record(err))),
                // With otherwise: success → unwrap value, no record wrap.
                (Ok(v), Some(_)) => force(v, &mut |e, env| evaluate(e, env, host)),
                // With otherwise: failure → evaluate fallback.
                (Err(_), Some(fb)) => {
                    let v = evaluate(fb, env, host)?;
                    force(v, &mut |e, env| evaluate(e, env, host))
                }
            }
        }

        // --- Forms not yet implemented ---
        Expr::ListType(_)
        | Expr::RecordType { .. }
        | Expr::TableType(_)
        | Expr::FunctionType { .. } => Err(MError::NotImplemented(
            "expression form deferred to a later eval slice",
        )),
    }
}

/// Build the standard `[Reason, Message, Detail]` error record used when
/// `error <text>` is invoked, and when internal MError variants are lifted
/// for `try` callers.
fn build_standard_error_record(reason: String, message: String) -> Value {
    let env = EnvNode::empty();
    Value::Record(Record {
        fields: vec![
            ("Reason".to_string(), Value::Text(reason)),
            ("Message".to_string(), Value::Text(message)),
            ("Detail".to_string(), Value::Null),
        ],
        env,
    })
}

/// `try` success-record builder: `[HasError = false, Value = v]`.
fn try_success_record(v: Value) -> Value {
    let env = EnvNode::empty();
    Value::Record(Record {
        fields: vec![
            ("HasError".to_string(), Value::Logical(false)),
            ("Value".to_string(), v),
        ],
        env,
    })
}

/// `try` failure-record builder: `[HasError = true, Error = <error-rec>]`.
fn try_failure_record(error_record: Value) -> Value {
    let env = EnvNode::empty();
    Value::Record(Record {
        fields: vec![
            ("HasError".to_string(), Value::Logical(true)),
            ("Error".to_string(), error_record),
        ],
        env,
    })
}

/// Lift an internal MError into a user-visible M error record so `try` can
/// surface it as a Value. User-raised errors pass through their inner record.
fn error_to_record(err: MError) -> Value {
    match err {
        MError::Raised(v) => v,
        MError::NameNotInScope(name) => build_standard_error_record(
            "Expression.Error".to_string(),
            format!("the name '{}' wasn't recognized", name),
        ),
        MError::TypeMismatch { expected, found } => build_standard_error_record(
            "Expression.Error".to_string(),
            format!("expected {}, found {}", expected, found),
        ),
        MError::NotImplemented(what) => build_standard_error_record(
            "Expression.Error".to_string(),
            format!("not implemented: {}", what),
        ),
        MError::Other(msg) => build_standard_error_record(
            "Expression.Error".to_string(),
            msg,
        ),
    }
}

/// Recursively force every thunk in a value tree. Internal thunks aren't
/// part of the user-visible value model — `value_dump`-style serializers
/// must walk and force lists/records before printing.
pub fn deep_force(value: Value, host: &dyn IoHost) -> Result<Value, MError> {
    let value = force(value, &mut |e, env| evaluate(e, env, host))?;
    match value {
        Value::List(items) => {
            let forced: Result<Vec<Value>, MError> = items
                .into_iter()
                .map(|v| deep_force(v, host))
                .collect();
            Ok(Value::List(forced?))
        }
        Value::Record(record) => {
            let fields: Result<Vec<(String, Value)>, MError> = record
                .fields
                .into_iter()
                .map(|(name, v)| Ok((name, deep_force(v, host)?)))
                .collect();
            Ok(Value::Record(Record {
                fields: fields?,
                env: record.env,
            }))
        }
        other => Ok(other),
    }
}

/// Raw lexeme → f64. Hex (0x.../0X...) goes through `u64::from_str_radix`
/// then casts; everything else through `f64::parse`.
fn parse_number_literal(lexeme: &str) -> Result<f64, MError> {
    if let Some(hex) = lexeme.strip_prefix("0x").or_else(|| lexeme.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16)
            .map(|n| n as f64)
            .map_err(|_| MError::Other(format!("invalid hex number: {}", lexeme)))
    } else {
        lexeme
            .parse::<f64>()
            .map_err(|_| MError::Other(format!("invalid number: {}", lexeme)))
    }
}

fn apply_unary(op: UnaryOp, v: Value) -> Result<Value, MError> {
    match op {
        UnaryOp::Plus => match v {
            Value::Number(n) => Ok(Value::Number(n)),
            other => Err(MError::TypeMismatch {
                expected: "number",
                found: type_name(&other),
            }),
        },
        UnaryOp::Minus => match v {
            Value::Number(n) => Ok(Value::Number(-n)),
            other => Err(MError::TypeMismatch {
                expected: "number",
                found: type_name(&other),
            }),
        },
        UnaryOp::Not => match v {
            Value::Logical(b) => Ok(Value::Logical(!b)),
            other => Err(MError::TypeMismatch {
                expected: "logical",
                found: type_name(&other),
            }),
        },
        UnaryOp::Type | UnaryOp::Nullable => Err(MError::NotImplemented(
            "type / nullable unary deferred to eval-5",
        )),
    }
}

fn apply_binary(op: BinaryOp, lv: Value, rv: Value) -> Result<Value, MError> {
    match op {
        BinaryOp::Multiply => arithmetic(lv, rv, |a, b| a * b),
        BinaryOp::Divide => arithmetic(lv, rv, |a, b| a / b),
        BinaryOp::Add => arithmetic(lv, rv, |a, b| a + b),
        BinaryOp::Subtract => arithmetic(lv, rv, |a, b| a - b),
        BinaryOp::Concat => match (lv, rv) {
            (Value::Text(l), Value::Text(r)) => Ok(Value::Text(l + &r)),
            // List concat is reachable only once eval-3 introduces list values.
            (l, _) => Err(MError::TypeMismatch {
                expected: "text",
                found: type_name(&l),
            }),
        },
        BinaryOp::LessThan => compare(lv, rv, std::cmp::Ordering::Less, false),
        BinaryOp::LessEquals => compare(lv, rv, std::cmp::Ordering::Less, true),
        BinaryOp::GreaterThan => compare(lv, rv, std::cmp::Ordering::Greater, false),
        BinaryOp::GreaterEquals => compare(lv, rv, std::cmp::Ordering::Greater, true),
        BinaryOp::Equal => Ok(Value::Logical(values_equal(&lv, &rv))),
        BinaryOp::NotEqual => Ok(Value::Logical(!values_equal(&lv, &rv))),
        BinaryOp::And | BinaryOp::Or => unreachable!("short-circuited above"),
        BinaryOp::As | BinaryOp::Is | BinaryOp::Meta => Err(MError::NotImplemented(
            "type relation / meta deferred to eval-5",
        )),
    }
}

fn arithmetic<F: FnOnce(f64, f64) -> f64>(
    lv: Value,
    rv: Value,
    op: F,
) -> Result<Value, MError> {
    let l = expect_number(&lv)?;
    let r = expect_number(&rv)?;
    // Divide by zero is positive/negative infinity per IEEE 754, not an error.
    Ok(Value::Number(op(l, r)))
}

fn expect_number(v: &Value) -> Result<f64, MError> {
    match v {
        Value::Number(n) => Ok(*n),
        other => Err(MError::TypeMismatch {
            expected: "number",
            found: type_name(other),
        }),
    }
}

fn compare(
    lv: Value,
    rv: Value,
    expected: std::cmp::Ordering,
    allow_equal: bool,
) -> Result<Value, MError> {
    let result = match (&lv, &rv) {
        (Value::Number(l), Value::Number(r)) => match l.partial_cmp(r) {
            Some(ord) => ord == expected || (allow_equal && ord == std::cmp::Ordering::Equal),
            // NaN involved — all comparisons return false per IEEE 754.
            None => false,
        },
        (Value::Text(l), Value::Text(r)) => {
            let ord = l.cmp(r);
            ord == expected || (allow_equal && ord == std::cmp::Ordering::Equal)
        }
        // For slice 1, cross-type and other-type comparisons error. Spec
        // covers more cases; they land with later slices as needed.
        _ => {
            return Err(MError::TypeMismatch {
                expected: "two numbers or two texts",
                found: type_name(&lv),
            });
        }
    };
    Ok(Value::Logical(result))
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Logical(x), Value::Logical(y)) => x == y,
        // Number equality follows IEEE 754: NaN != NaN. f64's PartialEq
        // matches that exactly.
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::Text(x), Value::Text(y)) => x == y,
        // Mismatched kinds are never equal in slice 1. Per spec there are
        // some allowed coercions; they land if/when the corpus needs them.
        _ => false,
    }
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Logical(_) => "logical",
        Value::Number(_) => "number",
        Value::Text(_) => "text",
        Value::Date(_) => "date",
        Value::Datetime(_) => "datetime",
        Value::Duration(_) => "duration",
        Value::Binary(_) => "binary",
        Value::List(_) => "list",
        Value::Record(_) => "record",
        Value::Table(_) => "table",
        Value::Function(_) => "function",
        Value::Type(_) => "type",
        Value::Thunk(_) => "thunk",
    }
}

/// Force a value: if it's a thunk, evaluate the deferred expression (using
/// the supplied evaluator callback), memoise the result, and recurse in case
/// the result is itself a thunk. Non-thunk values pass through unchanged.
///
/// The evaluator callback parameterisation breaks what would otherwise be a
/// circular dependency (force ↔ evaluate). Slice-1's `evaluate` will pass a
/// closure that recursively calls itself. Tests can pass a fake closure that
/// counts invocations to verify memoisation.
pub fn force<F>(value: Value, evaluator: &mut F) -> Result<Value, MError>
where
    F: FnMut(&Expr, &Env) -> Result<Value, MError>,
{
    let thunk_state = match &value {
        Value::Thunk(state) => Rc::clone(state),
        _ => return Ok(value),
    };

    // Fast path: already forced.
    {
        let borrowed = thunk_state.borrow();
        if let ThunkState::Forced(v) = &*borrowed {
            return Ok(v.clone());
        }
    }

    // Pending — extract the captured expr/env and evaluate.
    let (expr, env_weak) = {
        let borrowed = thunk_state.borrow();
        match &*borrowed {
            ThunkState::Pending { expr, env } => (expr.clone(), env.clone()),
            ThunkState::Forced(_) => unreachable!("just checked above"),
        }
    };
    let env = env_weak.upgrade().ok_or_else(|| {
        MError::Other("thunk's environment was dropped before forcing".into())
    })?;

    let result = evaluator(&expr, &env)?;
    let result = force(result, evaluator)?; // chained thunks

    *thunk_state.borrow_mut() = ThunkState::Forced(result.clone());
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn eval_str(src: &str) -> Result<Value, MError> {
        let toks = tokenize(src).expect("lex");
        let ast = parse(&toks).expect("parse");
        let env = EnvNode::empty();
        let host = NoIoHost;
        evaluate(&ast, &env, &host)
    }

    fn eval_number(src: &str) -> f64 {
        match eval_str(src).expect("eval") {
            Value::Number(n) => n,
            other => panic!("expected number, got {:?}", other),
        }
    }

    fn eval_text(src: &str) -> String {
        match eval_str(src).expect("eval") {
            Value::Text(s) => s,
            other => panic!("expected text, got {:?}", other),
        }
    }

    fn eval_bool(src: &str) -> bool {
        match eval_str(src).expect("eval") {
            Value::Logical(b) => b,
            other => panic!("expected logical, got {:?}", other),
        }
    }

    // --- Literals ---

    #[test]
    fn literal_integer() {
        assert_eq!(eval_number("42"), 42.0);
    }

    #[test]
    fn literal_decimal() {
        assert_eq!(eval_number("3.14"), 3.14);
    }

    #[test]
    fn literal_hex() {
        assert_eq!(eval_number("0xff"), 255.0);
        assert_eq!(eval_number("0X10"), 16.0);
    }

    #[test]
    fn literal_text() {
        assert_eq!(eval_text(r#""hello""#), "hello");
    }

    #[test]
    fn literal_logicals_and_null() {
        assert_eq!(eval_bool("true"), true);
        assert_eq!(eval_bool("false"), false);
        assert!(matches!(eval_str("null").unwrap(), Value::Null));
    }

    // --- Unary operators ---

    #[test]
    fn unary_negation() {
        assert_eq!(eval_number("-5"), -5.0);
        assert_eq!(eval_number("--5"), 5.0);
    }

    #[test]
    fn unary_plus_is_identity() {
        assert_eq!(eval_number("+7"), 7.0);
    }

    #[test]
    fn unary_not() {
        assert_eq!(eval_bool("not true"), false);
        assert_eq!(eval_bool("not false"), true);
    }

    #[test]
    fn unary_not_on_non_logical_errors() {
        match eval_str("not 5") {
            Err(MError::TypeMismatch {
                expected: "logical",
                found: "number",
            }) => {}
            other => panic!("expected TypeMismatch, got {:?}", other),
        }
    }

    // --- Arithmetic ---

    #[test]
    fn arithmetic_basic() {
        assert_eq!(eval_number("1 + 2"), 3.0);
        assert_eq!(eval_number("5 - 3"), 2.0);
        assert_eq!(eval_number("4 * 6"), 24.0);
        assert_eq!(eval_number("10 / 4"), 2.5);
    }

    #[test]
    fn arithmetic_precedence() {
        assert_eq!(eval_number("1 + 2 * 3"), 7.0);
        assert_eq!(eval_number("(1 + 2) * 3"), 9.0);
    }

    #[test]
    fn divide_by_zero_is_infinity() {
        let v = eval_number("1 / 0");
        assert!(v.is_infinite() && v > 0.0);
        let v = eval_number("-1 / 0");
        assert!(v.is_infinite() && v < 0.0);
    }

    #[test]
    fn arithmetic_type_mismatch() {
        match eval_str(r#""hi" + 1"#) {
            Err(MError::TypeMismatch { expected: "number", .. }) => {}
            other => panic!("expected TypeMismatch, got {:?}", other),
        }
    }

    // --- Concat ---

    #[test]
    fn text_concat() {
        assert_eq!(eval_text(r#""hello" & " " & "world""#), "hello world");
    }

    #[test]
    fn concat_type_mismatch_for_slice_1() {
        // Number-number concat is invalid (& is text/list only).
        assert!(eval_str("1 & 2").is_err());
    }

    // --- Comparison ---

    #[test]
    fn comparison_numbers() {
        assert_eq!(eval_bool("1 < 2"), true);
        assert_eq!(eval_bool("2 < 1"), false);
        assert_eq!(eval_bool("1 <= 1"), true);
        assert_eq!(eval_bool("2 > 1"), true);
        assert_eq!(eval_bool("2 >= 2"), true);
    }

    #[test]
    fn comparison_texts() {
        assert_eq!(eval_bool(r#""a" < "b""#), true);
        assert_eq!(eval_bool(r#""z" > "a""#), true);
    }

    #[test]
    fn comparison_cross_type_errors() {
        // Slice-1 strict: comparing number to text errors.
        assert!(eval_str(r#"1 < "x""#).is_err());
    }

    // --- Equality ---

    #[test]
    fn equality_basic() {
        assert_eq!(eval_bool("1 = 1"), true);
        assert_eq!(eval_bool("1 = 2"), false);
        assert_eq!(eval_bool("1 <> 2"), true);
        assert_eq!(eval_bool(r#""a" = "a""#), true);
        assert_eq!(eval_bool("true = true"), true);
        assert_eq!(eval_bool("null = null"), true);
    }

    #[test]
    fn equality_cross_type_returns_false() {
        // Per design: mismatched-kind equality returns false (no coercion in
        // slice 1; spec has some allowed coercions for later slices).
        assert_eq!(eval_bool(r#"1 = "1""#), false);
        assert_eq!(eval_bool(r#"1 <> "1""#), true);
        assert_eq!(eval_bool("null = false"), false);
    }

    #[test]
    fn nan_is_not_equal_to_itself() {
        // Per IEEE 754; matches f64's PartialEq.
        assert_eq!(eval_bool("(1 / 0 - 1 / 0) = (1 / 0 - 1 / 0)"), false);
    }

    // --- Logical ---

    #[test]
    fn logical_and_short_circuit() {
        // If left is false, right is never evaluated — even if it would error.
        assert_eq!(eval_bool("false and (1 / 0 > 0)"), false);
        // The TypeMismatch on the right side does NOT fire because we short-circuit.
        assert_eq!(eval_bool(r#"false and "not a logical""#), false);
    }

    #[test]
    fn logical_or_short_circuit() {
        assert_eq!(eval_bool("true or (1 / 0 > 0)"), true);
        assert_eq!(eval_bool(r#"true or "not a logical""#), true);
    }

    #[test]
    fn logical_combined() {
        assert_eq!(eval_bool("true and false"), false);
        assert_eq!(eval_bool("true and true"), true);
        assert_eq!(eval_bool("false or true"), true);
    }

    // --- if / then / else ---

    #[test]
    fn if_then_else_branches() {
        assert_eq!(eval_number("if true then 1 else 2"), 1.0);
        assert_eq!(eval_number("if false then 1 else 2"), 2.0);
    }

    #[test]
    fn if_lazy_branches() {
        // The not-taken branch is never evaluated.
        assert_eq!(eval_number("if true then 1 else (1 / 0 + 0)"), 1.0);
        assert_eq!(eval_number(r#"if false then "bad" + 1 else 2"#), 2.0);
    }

    #[test]
    fn if_cond_must_be_logical() {
        match eval_str("if 1 then 1 else 2") {
            Err(MError::TypeMismatch { expected: "logical", .. }) => {}
            other => panic!("expected TypeMismatch, got {:?}", other),
        }
    }

    // --- let / in ---

    #[test]
    fn let_single_binding() {
        assert_eq!(eval_number("let x = 7 in x"), 7.0);
    }

    #[test]
    fn let_uses_binding_in_body() {
        assert_eq!(eval_number("let x = 1 + 2 in x * 10"), 30.0);
    }

    #[test]
    fn let_sequential_visibility() {
        // Standard sequential let: b sees a.
        assert_eq!(eval_number("let a = 1, b = a + 1 in b"), 2.0);
    }

    #[test]
    fn let_mutual_visibility() {
        // The killer test: a is defined in terms of b, b in terms of nothing
        // (literal 1). The thunks for a and b share the same env, so a's
        // forcing finds b.
        assert_eq!(eval_number("let a = b + 1, b = 1 in a"), 2.0);
    }

    #[test]
    fn let_lazy_unused_error_doesnt_propagate() {
        // The killer property of laziness: an unused binding that *would*
        // error never raises.
        assert_eq!(eval_number(r#"let bad = "x" + 1, good = 1 in good"#), 1.0);
    }

    #[test]
    fn let_combined_with_if_and_arithmetic() {
        // The headline interaction test from the task description.
        assert_eq!(
            eval_number("let a = 1, b = a + 1 in if b > a then b else a"),
            2.0
        );
    }

    #[test]
    fn nested_let_shadows() {
        assert_eq!(eval_number("let x = 1 in let x = 2 in x"), 2.0);
        assert_eq!(eval_number("let x = 1 in (let x = 2 in x) + x"), 3.0);
    }

    // --- Errors ---

    #[test]
    fn name_not_in_scope() {
        match eval_str("missing") {
            Err(MError::NameNotInScope(n)) => assert_eq!(n, "missing"),
            other => panic!("expected NameNotInScope, got {:?}", other),
        }
    }

    #[test]
    fn deferred_form_returns_not_implemented() {
        // Type-expression forms aren't in slices 1-4 — eval-5 lands them.
        // Build the AST directly because they only appear inside `type X`
        // contexts that aren't ergonomically reachable from source.
        use crate::parser::Expr;
        let ast = Expr::ListType(Box::new(Expr::NumberLit("1".into())));
        let env = EnvNode::empty();
        match evaluate(&ast, &env, &NoIoHost) {
            Err(MError::NotImplemented(_)) => {}
            other => panic!("expected NotImplemented, got {:?}", other),
        }
    }

    // --- Env operations (kept from #33) ---

    #[test]
    fn lookup_finds_immediate_binding() {
        let env = EnvNode::empty().extend("x".into(), Value::Number(1.0));
        match env.lookup("x") {
            Some(Value::Number(n)) => assert_eq!(n, 1.0),
            other => panic!("expected Number(1.0), got {:?}", other),
        }
    }

    #[test]
    fn lookup_walks_parent_chain() {
        let outer = EnvNode::empty().extend("x".into(), Value::Number(1.0));
        let inner = outer.extend("y".into(), Value::Number(2.0));
        // y is in inner, x must be reached via parent chain
        assert!(matches!(inner.lookup("y"), Some(Value::Number(_))));
        assert!(matches!(inner.lookup("x"), Some(Value::Number(_))));
    }

    #[test]
    fn lookup_returns_none_for_missing() {
        let env = EnvNode::empty().extend("x".into(), Value::Number(1.0));
        assert!(env.lookup("y").is_none());
    }

    #[test]
    fn extend_shadows_parent() {
        let outer = EnvNode::empty().extend("x".into(), Value::Number(1.0));
        let inner = outer.extend("x".into(), Value::Number(2.0));
        // inner's x shadows outer's
        match inner.lookup("x") {
            Some(Value::Number(n)) => assert_eq!(n, 2.0),
            other => panic!("expected Number(2.0), got {:?}", other),
        }
        // outer is unaffected
        match outer.lookup("x") {
            Some(Value::Number(n)) => assert_eq!(n, 1.0),
            other => panic!("expected outer Number(1.0), got {:?}", other),
        }
    }

    // --- Force semantics ---

    /// A trivial fake evaluator that returns `Value::Number(42.0)` and
    /// counts invocations via a mutable counter the test owns.
    fn counting_evaluator(counter: &std::cell::Cell<u32>) -> impl FnMut(&Expr, &Env) -> Result<Value, MError> + '_ {
        move |_expr: &Expr, _env: &Env| {
            counter.set(counter.get() + 1);
            Ok(Value::Number(42.0))
        }
    }

    #[test]
    fn force_passes_through_non_thunks() {
        let counter = std::cell::Cell::new(0);
        let mut eval = counting_evaluator(&counter);
        match force(Value::Number(7.0), &mut eval) {
            Ok(Value::Number(n)) => assert_eq!(n, 7.0),
            other => panic!("expected Number(7.0), got {:?}", other),
        }
        assert_eq!(counter.get(), 0, "evaluator should not have been called");
    }

    #[test]
    fn force_evaluates_pending_thunk() {
        let counter = std::cell::Cell::new(0);
        let mut eval = counting_evaluator(&counter);

        // Build an env with a single lazy binding. Use a trivial expression
        // (the evaluator ignores it anyway).
        let toks = tokenize("ignored").unwrap();
        let ast = parse(&toks).unwrap();
        let env = EnvNode::empty().extend_lazy(vec![("x".into(), ast)]);

        let value = env.lookup("x").expect("x bound");
        match force(value, &mut eval) {
            Ok(Value::Number(n)) => assert_eq!(n, 42.0),
            other => panic!("expected Number(42.0), got {:?}", other),
        }
        assert_eq!(counter.get(), 1, "evaluator should have been called once");
    }

    #[test]
    fn force_memoises_pending_thunks() {
        let counter = std::cell::Cell::new(0);
        let mut eval = counting_evaluator(&counter);

        let toks = tokenize("ignored").unwrap();
        let ast = parse(&toks).unwrap();
        let env = EnvNode::empty().extend_lazy(vec![("x".into(), ast)]);

        // First force: evaluator called.
        let v1 = env.lookup("x").unwrap();
        force(v1, &mut eval).unwrap();
        assert_eq!(counter.get(), 1);

        // Second force of the same thunk: should hit the memoised cache.
        // (lookup() clones the Value, but Value::Thunk(Rc<RefCell<...>>)
        // shares the underlying ThunkState — both Rcs point at the same cell.)
        let v2 = env.lookup("x").unwrap();
        force(v2, &mut eval).unwrap();
        assert_eq!(counter.get(), 1, "second force should hit memoised cache");
    }

    // --- Functions, invocation, each, @ self-reference (eval-2) ---

    #[test]
    fn invoke_identity_function() {
        assert_eq!(eval_number("((x) => x)(42)"), 42.0);
    }

    #[test]
    fn invoke_multi_arg() {
        assert_eq!(eval_number("((x, y) => x + y)(3, 4)"), 7.0);
    }

    #[test]
    fn invoke_optional_missing_is_null() {
        assert_eq!(
            eval_number("((x, optional y) => if y = null then x else x + y)(5)"),
            5.0
        );
    }

    #[test]
    fn invoke_optional_supplied() {
        assert_eq!(
            eval_number("((x, optional y) => if y = null then x else x + y)(5, 10)"),
            15.0
        );
    }

    #[test]
    fn invoke_arity_too_few() {
        match eval_str("((x, y) => x + y)(1)") {
            Err(MError::Other(msg)) => assert!(msg.contains("arity"), "got: {}", msg),
            other => panic!("expected arity error, got {:?}", other),
        }
    }

    #[test]
    fn invoke_arity_too_many() {
        match eval_str("((x) => x)(1, 2)") {
            Err(MError::Other(msg)) => assert!(msg.contains("arity"), "got: {}", msg),
            other => panic!("expected arity error, got {:?}", other),
        }
    }

    #[test]
    fn invoke_non_function() {
        match eval_str("(42)(1)") {
            Err(MError::TypeMismatch {
                expected: "function",
                found: "number",
            }) => {}
            other => panic!("expected TypeMismatch on non-function, got {:?}", other),
        }
    }

    #[test]
    fn each_desugars_to_underscore_lambda() {
        assert_eq!(eval_number("(each _ + 1)(5)"), 6.0);
    }

    #[test]
    fn closure_captures_outer_let_binding() {
        assert_eq!(eval_number("let n = 10 in ((x) => x + n)(5)"), 15.0);
    }

    #[test]
    fn nested_closure_currying() {
        assert_eq!(eval_number("((x) => (y) => x + y)(3)(4)"), 7.0);
    }

    #[test]
    fn recursive_function_via_at_self_reference() {
        // Killer test: the @fact reference inside the function body resolves
        // against the let env that contains the function's own binding.
        // Thunk-in-same-env design means @ is just regular lookup.
        assert_eq!(
            eval_number(
                "let fact = (n) => if n <= 1 then 1 else n * @fact(n - 1) in fact(5)"
            ),
            120.0
        );
    }

    #[test]
    fn mutually_recursive_functions_via_at() {
        assert_eq!(
            eval_bool(
                "let even = (n) => if n = 0 then true else @odd(n - 1), \
                     odd = (n) => if n = 0 then false else @even(n - 1) \
                 in even(4)"
            ),
            true
        );
    }

    // --- Lists, records, field/item access (eval-3) ---

    fn eval_list_len(src: &str) -> usize {
        match eval_str(src).expect("eval") {
            Value::List(xs) => xs.len(),
            other => panic!("expected list, got {:?}", other),
        }
    }

    #[test]
    fn empty_list() {
        assert_eq!(eval_list_len("{}"), 0);
    }

    #[test]
    fn single_item_list() {
        match eval_str("{42}").unwrap() {
            Value::List(xs) => match xs.as_slice() {
                [Value::Number(n)] => assert_eq!(*n, 42.0),
                other => panic!("unexpected items: {:?}", other),
            },
            other => panic!("expected list, got {:?}", other),
        }
    }

    #[test]
    fn multi_item_list() {
        assert_eq!(eval_list_len("{1, 2, 3, 4, 5}"), 5);
    }

    #[test]
    fn range_expands_to_inclusive_integers() {
        match eval_str("{1..3}").unwrap() {
            Value::List(xs) => {
                let nums: Vec<f64> = xs
                    .iter()
                    .map(|v| match v {
                        Value::Number(n) => *n,
                        other => panic!("expected number, got {:?}", other),
                    })
                    .collect();
                assert_eq!(nums, vec![1.0, 2.0, 3.0]);
            }
            other => panic!("expected list, got {:?}", other),
        }
    }

    #[test]
    fn range_with_non_integer_bounds_errors() {
        assert!(eval_str("{1.5..3}").is_err());
    }

    #[test]
    fn range_descending_errors() {
        assert!(eval_str("{5..1}").is_err());
    }

    #[test]
    fn empty_record() {
        match eval_str("[]").unwrap() {
            Value::Record(r) => assert!(r.fields.is_empty()),
            other => panic!("expected record, got {:?}", other),
        }
    }

    #[test]
    fn record_field_access() {
        assert_eq!(eval_number("[a = 1, b = 2][b]"), 2.0);
    }

    #[test]
    fn record_sibling_reference() {
        // b references a — possible because both are thunks in the same env.
        assert_eq!(eval_number("[a = 1, b = a + 1][b]"), 2.0);
    }

    #[test]
    fn record_lazy_unused_error_field() {
        // bad would error if forced (missing_name is unbound), but [good] never
        // touches it. Laziness applies per-field for records.
        assert_eq!(
            eval_number("[bad = missing_name, good = 1][good]"),
            1.0
        );
    }

    #[test]
    fn field_access_optional_missing_is_null() {
        match eval_str("[a = 1][missing]?").unwrap() {
            Value::Null => {}
            other => panic!("expected null, got {:?}", other),
        }
    }

    #[test]
    fn field_access_optional_present_returns_value() {
        assert_eq!(eval_number("[a = 1][a]?"), 1.0);
    }

    #[test]
    fn field_access_required_missing_errors() {
        match eval_str("[a = 1][missing]") {
            Err(MError::Other(msg)) => assert!(msg.contains("field not found"), "got: {}", msg),
            other => panic!("expected field-not-found error, got {:?}", other),
        }
    }

    #[test]
    fn item_access_on_list() {
        assert_eq!(eval_number("{10, 20, 30}{1}"), 20.0);
    }

    #[test]
    fn item_access_zero_index() {
        assert_eq!(eval_number("{10, 20, 30}{0}"), 10.0);
    }

    #[test]
    fn item_access_out_of_bounds_optional() {
        match eval_str("{10, 20}{99}?").unwrap() {
            Value::Null => {}
            other => panic!("expected null, got {:?}", other),
        }
    }

    #[test]
    fn item_access_out_of_bounds_required_errors() {
        assert!(eval_str("{10, 20}{99}").is_err());
    }

    #[test]
    fn record_in_let_then_field_access() {
        assert_eq!(
            eval_number("let r = [a = 1, b = a + 10] in r[b]"),
            11.0
        );
    }

    #[test]
    fn implicit_underscore_access_in_lambda() {
        // `[name]` as a primary desugars to `_[name]` — the lambda's _ param
        // is the record we're accessing.
        match eval_str(r#"(each [name])([name = "ok"])"#).unwrap() {
            Value::Text(s) => assert_eq!(s, "ok"),
            other => panic!("expected text, got {:?}", other),
        }
    }

    #[test]
    fn nested_list_access() {
        assert_eq!(eval_number("{{1, 2}, {3, 4}}{0}{1}"), 2.0);
    }

    // --- try / otherwise / error (eval-4) ---

    fn record_field(v: &Value, name: &str) -> Value {
        match v {
            Value::Record(r) => r
                .fields
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| panic!("field {} not found in {:?}", name, v)),
            other => panic!("expected record, got {:?}", other),
        }
    }

    #[test]
    fn try_success_no_otherwise_wraps() {
        let v = eval_str("try 42").unwrap();
        let v = deep_force(v, &NoIoHost).unwrap();
        match &record_field(&v, "HasError") {
            Value::Logical(false) => {}
            other => panic!("expected HasError=false, got {:?}", other),
        }
        match &record_field(&v, "Value") {
            Value::Number(n) => assert_eq!(*n, 42.0),
            other => panic!("expected Value=42, got {:?}", other),
        }
    }

    #[test]
    fn try_failure_no_otherwise_wraps() {
        let v = eval_str("try missing_name").unwrap();
        let v = deep_force(v, &NoIoHost).unwrap();
        match &record_field(&v, "HasError") {
            Value::Logical(true) => {}
            other => panic!("expected HasError=true, got {:?}", other),
        }
        let err = record_field(&v, "Error");
        // Error.Message should mention the missing name
        match record_field(&err, "Message") {
            Value::Text(s) => assert!(s.contains("missing_name"), "got: {}", s),
            other => panic!("expected text Message, got {:?}", other),
        }
    }

    #[test]
    fn try_otherwise_success_returns_value() {
        assert_eq!(eval_number("try 42 otherwise 0"), 42.0);
    }

    #[test]
    fn try_otherwise_failure_returns_fallback() {
        assert_eq!(eval_number("try missing_name otherwise 99"), 99.0);
    }

    #[test]
    fn error_text_propagates_as_raised() {
        match eval_str(r#"error "boom""#) {
            Err(MError::Raised(_)) => {}
            other => panic!("expected MError::Raised, got {:?}", other),
        }
    }

    #[test]
    fn try_catches_user_error_text() {
        let v = eval_str(r#"try error "boom""#).unwrap();
        let v = deep_force(v, &NoIoHost).unwrap();
        match &record_field(&v, "HasError") {
            Value::Logical(true) => {}
            other => panic!("expected HasError=true, got {:?}", other),
        }
        let err = record_field(&v, "Error");
        match record_field(&err, "Message") {
            Value::Text(s) => assert_eq!(s, "boom"),
            other => panic!("expected Message=boom, got {:?}", other),
        }
        match record_field(&err, "Reason") {
            Value::Text(s) => assert_eq!(s, "Expression.Error"),
            other => panic!("expected Reason=Expression.Error, got {:?}", other),
        }
    }

    #[test]
    fn try_catches_user_error_record() {
        let v = eval_str(
            r#"try error [Reason = "X", Message = "Y", Detail = null]"#,
        )
        .unwrap();
        let v = deep_force(v, &NoIoHost).unwrap();
        let err = record_field(&v, "Error");
        match record_field(&err, "Reason") {
            Value::Text(s) => assert_eq!(s, "X"),
            other => panic!("expected Reason=X, got {:?}", other),
        }
        match record_field(&err, "Message") {
            Value::Text(s) => assert_eq!(s, "Y"),
            other => panic!("expected Message=Y, got {:?}", other),
        }
    }

    #[test]
    fn error_propagates_through_arithmetic() {
        // error "x" + 1: the error fires when the left operand is evaluated.
        // The right operand is never reached; the whole expression errors.
        assert_eq!(eval_number(r#"try (error "x" + 1) otherwise 99"#), 99.0);
    }

    #[test]
    fn try_preserves_lazy_unforced_bindings() {
        // The let body returns 1; `bad` is never forced. The try should see
        // no error.
        assert_eq!(
            eval_number("try (let bad = missing_name in 1) otherwise 99"),
            1.0
        );
    }

    #[test]
    fn nested_try() {
        // Inner try catches "inner", returns a HasError record. Outer try
        // sees that as a successful (record-valued) result, so it returns
        // the record. With outer "otherwise 7", we get the record back as
        // the value, not 7 — because the inner try CAUGHT the error.
        // To test nested-error propagation, we need the outer try's body
        // itself to raise. Use: `try error "outer" otherwise 7` = 7.
        assert_eq!(eval_number(r#"try error "outer" otherwise 7"#), 7.0);
    }

    #[test]
    fn function_literal_evaluates_to_function_value() {
        match eval_str("(x) => x + 1") {
            Ok(Value::Function(_)) => {}
            other => panic!("expected Function value, got {:?}", other),
        }
    }

    #[test]
    fn extend_lazy_supports_mutual_recursion() {
        // Thunks for `a` and `b` share the same env, so when `a`'s thunk is
        // forced and looks up `b`, it resolves to `b`'s thunk in the same
        // frame. We don't have a real evaluator yet so we fake it: the
        // evaluator we pass in just returns a sentinel proving it was called
        // with the *same* env that contains both bindings.
        //
        // The point of this test: prove the mutual-visibility property holds
        // structurally, before slice-1 wires up a real recursive evaluator.
        let toks = tokenize("ignored").unwrap();
        let ast_a = parse(&toks).unwrap();
        let ast_b = parse(&toks).unwrap();
        let env = EnvNode::empty().extend_lazy(vec![
            ("a".into(), ast_a),
            ("b".into(), ast_b),
        ]);

        // From within the env (when a thunk is forced), both names resolve.
        // We simulate the "during force" lookup by directly looking up in env.
        assert!(env.lookup("a").is_some());
        assert!(env.lookup("b").is_some());

        // Verify the thunks share the same env via Weak refs that upgrade
        // successfully (i.e. point at the env that's still alive).
        let a_value = env.lookup("a").unwrap();
        if let Value::Thunk(state) = a_value {
            let borrowed = state.borrow();
            if let ThunkState::Pending { env: weak_env, .. } = &*borrowed {
                let upgraded = weak_env.upgrade().expect("env still alive");
                // The upgraded env should be the same pointer as our env.
                assert!(Rc::ptr_eq(&upgraded, &env), "thunk's env must be the extended env");
            } else {
                panic!("expected Pending thunk state");
            }
        } else {
            panic!("expected Thunk value");
        }
    }
}
