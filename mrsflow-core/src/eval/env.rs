//! Environment representation: persistent linked frames.
//!
//! `Env` is a reference-counted pointer to an `EnvNode`. Each node carries
//! its own bindings plus an optional pointer to its enclosing scope. Lookup
//! walks the chain; extending produces a new node whose `parent` points at
//! the current one, so sharing is automatic and closures can cheaply capture
//! the env they were defined in.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::parser::Expr;

use super::value::{ThunkState, Value};

pub type Env = Rc<EnvNode>;

#[derive(Debug)]
pub struct EnvNode {
    pub bindings: HashMap<String, Value>,
    pub parent: Option<Env>,
}

impl EnvNode {
    /// Empty root environment.
    pub fn empty() -> Env {
        Rc::new(EnvNode {
            bindings: HashMap::new(),
            parent: None,
        })
    }
}

/// Operations on an env. Implemented for the type alias `Env = Rc<EnvNode>`.
pub trait EnvOps {
    /// Walk the parent chain looking for `name`. Returns the bound value
    /// (cloned — Value variants are designed to clone cheaply, mostly Rc-wrapped).
    /// Caller is responsible for forcing if the result is a `Value::Thunk`.
    fn lookup(&self, name: &str) -> Option<Value>;

    /// Extend with a single eager binding. Returns a new env whose parent
    /// is `self`. Useful for function-call argument binding.
    fn extend(&self, name: String, value: Value) -> Env;

    /// Extend with multiple lazy bindings — each binding becomes a thunk
    /// that references the *new* env, so mutual recursion works:
    /// `let a = b + 1, b = 1 in a` evaluates to 2 because the thunk for `a`
    /// can resolve `b` against the same frame it lives in.
    fn extend_lazy(&self, bindings: Vec<(String, Expr)>) -> Env;
}

impl EnvOps for Env {
    fn lookup(&self, name: &str) -> Option<Value> {
        let mut current: Option<&EnvNode> = Some(self);
        while let Some(node) = current {
            if let Some(value) = node.bindings.get(name) {
                #[cfg(feature = "profile-clones")]
                {
                    use super::value::profile;
                    use std::sync::atomic::Ordering;
                    profile::ENV_LOOKUPS.fetch_add(1, Ordering::Relaxed);
                    if let Value::List(xs) = value {
                        profile::ENV_LIST_LOOKUPS.fetch_add(1, Ordering::Relaxed);
                        profile::ENV_LIST_LOOKUP_TOTAL_LEN
                            .fetch_add(xs.len() as u64, Ordering::Relaxed);
                    }
                }
                return Some(value.clone());
            }
            current = node.parent.as_deref();
        }
        None
    }

    fn extend(&self, name: String, value: Value) -> Env {
        let mut bindings = HashMap::new();
        bindings.insert(name, value);
        Rc::new(EnvNode {
            bindings,
            parent: Some(Rc::clone(self)),
        })
    }

    fn extend_lazy(&self, bindings: Vec<(String, Expr)>) -> Env {
        // `Rc::new_cyclic` provides a `Weak<Self>` during construction —
        // exactly what we need so each thunk can reference the env it lives
        // in without a strong-cycle leak.
        let parent = Rc::clone(self);
        Rc::new_cyclic(move |weak: &Weak<EnvNode>| {
            let mut binding_map = HashMap::new();
            for (name, expr) in bindings {
                let thunk = Value::Thunk(Rc::new(RefCell::new(ThunkState::Pending {
                    expr,
                    env: weak.clone(),
                })));
                binding_map.insert(name, thunk);
            }
            EnvNode {
                bindings: binding_map,
                parent: Some(parent),
            }
        })
    }
}
