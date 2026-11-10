//! The values a specification computes with, and the environment that holds its variables.

use std::collections::HashMap;
use std::fmt;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum Value {
    Int(i64),
    Real(f64),
    Bool(bool),
    Str(String),
    Arr {
        from: i64,
        items: Arc<Vec<Value>>,
    },
    Rec(Arc<Vec<(String, Value)>>),
    Tuple(Vec<Value>, &'static [&'static str]),
    /// Result slot with no meaning (index of a failed SEARCH); equal to anything.
    Undef,
}

pub type Env = HashMap<String, Value, BuildHasherDefault<NameHasher>>;

#[derive(Clone, Copy)]
pub struct NameHasher(u64);

impl Default for NameHasher {
    fn default() -> Self {
        NameHasher(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for NameHasher {
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

impl Value {
    pub fn eq_val(&self, o: &Value) -> bool {
        use Value::*;
        match (self, o) {
            (Undef, _) | (_, Undef) => true,
            (Int(a), Int(b)) => a == b,
            (Int(_) | Real(_), Int(_) | Real(_)) => (self.num() - o.num()).abs() < 1e-9,
            (Bool(a), Bool(b)) => a == b,
            (Str(a), Str(b)) => a == b,
            (Arr { items: a, .. }, Arr { items: b, .. }) => Arc::ptr_eq(a, b) || all_equal(a, b),
            (Tuple(a, _), Tuple(b, _)) => all_equal(a, b),
            (Rec(a), Rec(b)) => {
                Arc::ptr_eq(a, b)
                    || a.len() == b.len()
                        && a.iter()
                            .all(|(k, v)| b.iter().any(|(k2, v2)| k == k2 && v.eq_val(v2)))
            }
            _ => false,
        }
    }
    pub(super) fn num(&self) -> f64 {
        match self {
            Value::Int(i) => *i as f64,
            Value::Real(r) => *r,
            _ => f64::NAN,
        }
    }
    pub(super) fn is_num(&self) -> bool {
        matches!(self, Value::Int(_) | Value::Real(_))
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Int(_) => "a whole number",
            Value::Real(_) => "a real number",
            Value::Bool(_) => "a logical value",
            Value::Str(_) => "a text",
            Value::Arr { .. } => "an array",
            Value::Rec(_) => "a record",
            Value::Tuple(..) => "a tuple",
            Value::Undef => "undefined",
        }
    }
    /// Short rendering for messages: long arrays are abbreviated.
    pub fn short(&self) -> String {
        match self {
            Value::Arr { items, .. } if items.len() > 12 => format!("[{} elements]", items.len()),
            _ => self.to_string(),
        }
    }
}

fn all_equal(a: &[Value], b: &[Value]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.eq_val(y))
}

const SUMMARY_ITEMS: usize = 100;

impl Value {
    pub fn summary(&self) -> String {
        struct Summary<'a>(&'a Value);
        impl fmt::Display for Summary<'_> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                self.0.write(f, SUMMARY_ITEMS)
            }
        }
        Summary(self).to_string()
    }

    fn write(&self, f: &mut fmt::Formatter, most: usize) -> fmt::Result {
        let list = |f: &mut fmt::Formatter, v: &[Value]| -> fmt::Result {
            for (i, x) in v.iter().take(most).enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                x.write(f, most)?;
            }
            if v.len() > most {
                write!(f, ", … ({} elements)", v.len())?;
            }
            Ok(())
        };
        match self {
            Value::Int(i) => write!(f, "{i}"),
            Value::Real(r) => write!(f, "{r}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Str(s) => write!(f, "\"{s}\""),
            Value::Arr { items, .. } => {
                write!(f, "[")?;
                list(f, items)?;
                write!(f, "]")
            }
            Value::Tuple(items, _) => {
                write!(f, "(")?;
                list(f, items)?;
                write!(f, ")")
            }
            Value::Rec(fs) => {
                write!(f, "{{")?;
                for (i, (k, v)) in fs.iter().enumerate() {
                    write!(f, "{}{k}: ", if i > 0 { ", " } else { "" })?;
                    v.write(f, most)?;
                }
                write!(f, "}}")
            }
            Value::Undef => write!(f, "undefined"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.write(f, usize::MAX)
    }
}

pub(super) fn set(env: &mut Env, name: &str, v: Value) -> Option<Value> {
    match env.get_mut(name) {
        Some(slot) => Some(std::mem::replace(slot, v)),
        None => env.insert(name.to_string(), v),
    }
}

/// Put back the value a loop variable or parameter shadowed.
pub(super) fn restore(env: &mut Env, name: &str, old: Option<Value>) {
    match old {
        Some(v) => {
            env.insert(name.to_string(), v);
        }
        None => {
            env.remove(name);
        }
    }
}

/// Give an array (and its nested rows) the start indexes `starts`, outermost first.
pub fn set_from(v: &mut Value, starts: &[i64]) {
    if let (Value::Arr { from, items }, Some((&f, rest))) = (v, starts.split_first()) {
        *from = f;
        if !rest.is_empty() {
            for it in Arc::make_mut(items) {
                set_from(it, rest);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ints(from: i64, v: &[i64]) -> Value {
        Value::Arr {
            from,
            items: Arc::new(v.iter().map(|i| Value::Int(*i)).collect()),
        }
    }

    #[test]
    fn equality_as_the_language_sees_it() {
        use Value::*;
        assert!(Int(2).eq_val(&Real(2.0)) && Real(0.1 + 0.2).eq_val(&Real(0.3)));
        assert!(!Int(2).eq_val(&Real(2.5)) && !Int(1).eq_val(&Bool(true)));
        assert!(!Str("1".into()).eq_val(&Int(1)));
        assert!(Undef.eq_val(&Int(7)) && Str("x".into()).eq_val(&Undef));
        assert!(ints(1, &[1, 2]).eq_val(&ints(0, &[1, 2])));
        assert!(!ints(1, &[1, 2]).eq_val(&ints(1, &[1, 2, 3])));
        assert!(ints(1, &[]).eq_val(&ints(5, &[])));
        let rec =
            |a: i64, b: &str| Rec(vec![("n".into(), Int(a)), ("s".into(), Str(b.into()))].into());
        let swapped = Rec(vec![("s".into(), Str("x".into())), ("n".into(), Int(1))].into());
        assert!(rec(1, "x").eq_val(&swapped) && !rec(2, "x").eq_val(&swapped));
        assert!(Tuple(vec![Int(1), Undef], &[]).eq_val(&Tuple(vec![Int(1), Int(9)], &["a", "b"])));
    }

    #[test]
    fn values_are_shown_like_test_data() {
        use Value::*;
        assert_eq!(ints(1, &[1, -2]).to_string(), "[1, -2]");
        assert_eq!(Str("a b".into()).to_string(), "\"a b\"");
        assert_eq!(
            Tuple(vec![Bool(true), Undef], &[]).to_string(),
            "(true, undefined)"
        );
        let rec = Rec(Arc::new(vec![
            ("name".into(), Str("L1".into())),
            ("x".into(), Real(1.5)),
        ]));
        assert_eq!(rec.to_string(), "{name: \"L1\", x: 1.5}");
        assert_eq!(ints(1, &[0; 13]).short(), "[13 elements]");
        assert_eq!(ints(1, &[0; 12]).short(), ints(1, &[0; 12]).to_string());
        let hundred = ints(1, &[7; 100]);
        assert_eq!(hundred.summary(), hundred.to_string());
        let long = ints(1, &[7; 101]);
        let shown = long.summary();
        assert!(shown.starts_with("[7, 7, ") && shown.ends_with("7, … (101 elements)]"));
        assert_eq!(shown.matches('7').count(), 100);
        let pair = Tuple(vec![Int(101), long.clone()], &[]);
        assert_eq!(pair.summary(), format!("(101, {shown})"));
        assert_eq!(long.to_string().matches('7').count(), 101);
    }

    #[test]
    fn variables_are_shadowed_and_restored() {
        let mut env = Env::default();
        assert!(set(&mut env, "i", Value::Int(1)).is_none());
        let old = set(&mut env, "i", Value::Int(2));
        assert!(matches!(old, Some(Value::Int(1))));
        assert!(matches!(env["i"], Value::Int(2)));
        restore(&mut env, "i", old);
        assert!(matches!(env["i"], Value::Int(1)));
        restore(&mut env, "i", None);
        assert!(env.is_empty());
        for n in ["a", "b", "ab", "ba", "szamok", "számok"] {
            set(&mut env, n, Value::Str(n.into()));
        }
        assert_eq!(env.len(), 6);
        assert!(matches!(&env["ba"], Value::Str(s) if s == "ba"));
    }

    #[test]
    fn start_indexes_apply_to_every_dimension() {
        let mut m = Value::Arr {
            from: 1,
            items: Arc::new(vec![ints(1, &[1, 2]), ints(1, &[3, 4])]),
        };
        set_from(&mut m, &[-1, 0]);
        let Value::Arr { from, items } = &m else {
            unreachable!()
        };
        assert_eq!(*from, -1);
        assert!(matches!(&items[1], Value::Arr { from: 0, .. }));
        set_from(&mut m, &[5]);
        let Value::Arr { from, items } = &m else {
            unreachable!()
        };
        assert_eq!(*from, 5);
        assert!(matches!(&items[0], Value::Arr { from: 0, .. }));
        let mut n = Value::Int(3);
        set_from(&mut n, &[2]);
        assert!(matches!(n, Value::Int(3)));
    }
}
