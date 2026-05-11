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
pub use value::{Closure, MError, Table, ThunkState, TypeRep, Value};

use std::rc::Rc;

use crate::parser::{BinaryOp, Expr, UnaryOp};

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

        // --- Forms not in slice 1 ---
        Expr::Function { .. }
        | Expr::Each(_)
        | Expr::Invoke { .. }
        | Expr::FieldAccess { .. }
        | Expr::ItemAccess { .. }
        | Expr::Try { .. }
        | Expr::Error(_)
        | Expr::Record(_)
        | Expr::List(_)
        | Expr::ListType(_)
        | Expr::RecordType { .. }
        | Expr::TableType(_)
        | Expr::FunctionType { .. } => Err(MError::NotImplemented(
            "expression form deferred to a later eval slice",
        )),
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
        // Records aren't in slice 1 — should error with NotImplemented, not panic.
        match eval_str("[a = 1]") {
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
