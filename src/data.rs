//! Test case data (YAML) → typed values.

use crate::diag::{Diagnostic, did_you_mean};
use crate::eval::{Env, Eval, Value};
use crate::parser::{Expr, Role, Spec, Type};
use yaml_rust2::{Yaml, YamlLoader};

fn show(y: &Yaml) -> String {
    match y {
        Yaml::Integer(i) => i.to_string(),
        Yaml::Real(s) | Yaml::String(s) => s.clone(),
        Yaml::Boolean(b) => b.to_string(),
        Yaml::Array(a) => format!("[{} element(s)]", a.len()),
        Yaml::Hash(_) => "{…}".into(),
        Yaml::Null => "nothing".into(),
        _ => "?".into(),
    }
}

/// Line (1-based) of `key:` at the start of a line in the data text.
fn key_line(text: &str, key: &str) -> usize {
    text.lines()
        .position(|l| {
            l.trim_start()
                .strip_prefix(key)
                .is_some_and(|r| r.trim_start().starts_with(':'))
        })
        .map(|i| i + 1)
        .unwrap_or(0)
}

pub fn type_name(spec: &Spec, t: &Type) -> String {
    match t {
        Type::Nat => "N".into(),
        Type::Int => "Z".into(),
        Type::Real => "R".into(),
        Type::Bool => "L".into(),
        Type::Str => "S".into(),
        Type::Char => "C".into(),
        Type::Interval(a, b) => format!("[{}..{}]", spec.text(a), spec.text(b)),
        Type::Array { lo, hi, elem } => {
            format!(
                "{}[{}..{}]",
                type_name(spec, elem),
                spec.text(lo),
                hi.as_ref().map(|h| spec.text(h)).unwrap_or("")
            )
        }
        Type::Record(fs) => format!(
            "({})",
            fs.iter()
                .map(|(n, t)| format!("{n}:{}", type_name(spec, t)))
                .collect::<Vec<_>>()
                .join(" x ")
        ),
        Type::Named(n) => n.clone(),
    }
}

struct Conv<'a, 'b> {
    spec: &'a Spec,
    ev: &'b mut Eval<'a>,
    env: &'b mut Env,
}

impl Conv<'_, '_> {
    fn bound(&mut self, e: &Expr) -> Option<i64> {
        match self.ev.eval(e, &mut self.env.clone()) {
            Ok(Value::Int(i)) => Some(i),
            _ => None,
        }
    }

    fn conv(&mut self, y: &Yaml, t: &Type, path: &str) -> Result<Value, String> {
        let spec = self.spec;
        let got = show(y);
        match t {
            Type::Named(n) => {
                let r = spec
                    .resolve(t)
                    .ok_or_else(|| format!("the type `{n}` is not defined"))?;
                self.conv(y, r, path)
            }
            Type::Nat => match y {
                Yaml::Integer(i) if *i >= 0 => Ok(Value::Int(*i)),
                Yaml::Integer(i) => Err(format!(
                    "`{path}` is {i}, but it is declared as N, which only contains whole numbers ≥ 0. If negative values are valid (e.g. a -1 \"not found\" answer), declare it as Z."
                )),
                _ => Err(format!(
                    "`{path}` should be a natural number (0, 1, 2, …), but it is `{got}`."
                )),
            },
            Type::Int => match y {
                Yaml::Integer(i) => Ok(Value::Int(*i)),
                _ => Err(format!(
                    "`{path}` should be a whole number (Z), but it is `{got}`."
                )),
            },
            Type::Real => match y {
                Yaml::Integer(i) => Ok(Value::Real(*i as f64)),
                Yaml::Real(s) => s
                    .parse()
                    .map(Value::Real)
                    .map_err(|_| format!("`{path}`: `{s}` is not a number.")),
                _ => Err(format!(
                    "`{path}` should be a real number (R), but it is `{got}`."
                )),
            },
            Type::Bool => match y {
                Yaml::Boolean(b) => Ok(Value::Bool(*b)),
                _ => Err(format!(
                    "`{path}` should be `true` or `false` (L), but it is `{got}`."
                )),
            },
            Type::Str => match y {
                Yaml::String(s) | Yaml::Real(s) => Ok(Value::Str(s.clone())),
                Yaml::Integer(i) => Ok(Value::Str(i.to_string())),
                Yaml::Boolean(b) => Ok(Value::Str(b.to_string())),
                Yaml::Null => Err(format!(
                    "`{path}` has no value. Write `\"\"` for an empty text."
                )),
                _ => Err(format!("`{path}` should be a text (S), but it is `{got}`.")),
            },
            Type::Char => {
                let s = match y {
                    Yaml::String(s) => s.clone(),
                    Yaml::Integer(i) => i.to_string(),
                    _ => String::new(),
                };
                if s.chars().count() == 1 {
                    Ok(Value::Str(s))
                } else {
                    Err(format!(
                        "`{path}` should be a single character (C), but it is `{got}`."
                    ))
                }
            }
            Type::Interval(lo, hi) => {
                let Yaml::Integer(i) = y else {
                    return Err(format!(
                        "`{path}` should be a whole number in [{}..{}], but it is `{got}`.",
                        spec.text(lo),
                        spec.text(hi)
                    ));
                };
                if let (Some(l), Some(h)) = (self.bound(lo), self.bound(hi))
                    && (*i < l || *i > h)
                {
                    return Err(format!(
                        "`{path}` is {i}, but it must be in [{}..{}] = [{l}..{h}].",
                        spec.text(lo),
                        spec.text(hi)
                    ));
                }
                Ok(Value::Int(*i))
            }
            Type::Array { lo, hi, elem } => {
                let empty = Vec::new();
                let items = match y {
                    Yaml::Array(a) => a,
                    Yaml::Null => &empty,
                    _ => {
                        return Err(format!(
                            "`{path}` should be an array like `[1, 2, 3]` (declared as {}), but it is `{got}`.",
                            type_name(spec, t)
                        ));
                    }
                };
                let from = self.bound(lo).ok_or_else(|| {
                    format!(
                        "the start index `{}` of `{path}` can't be computed from the test data",
                        spec.text(lo)
                    )
                })?;
                if let Some(h) = hi
                    && let Some(to) = self.bound(h)
                {
                    let want = (to - from + 1).max(0);
                    if items.len() as i64 != want {
                        let txt = spec.text(h);
                        let why = if txt.parse::<i64>().is_ok() {
                            String::new()
                        } else {
                            format!(" and {txt} = {to}")
                        };
                        return Err(format!(
                            "`{path}` has {} element(s), but it is declared as {}{why}, so it needs exactly {want}.",
                            items.len(),
                            type_name(spec, t)
                        ));
                    }
                }
                let mut v = Vec::new();
                for (k, item) in items.iter().enumerate() {
                    v.push(self.conv(item, elem, &format!("{path}[{}]", from + k as i64))?);
                }
                Ok(Value::Arr { from, items: v })
            }
            Type::Record(fields) => {
                let Yaml::Hash(h) = y else {
                    let names: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
                    return Err(format!(
                        "`{path}` should be a record like `{{{}}}`, but it is `{got}`.",
                        names
                            .iter()
                            .map(|n| format!("{n}: …"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                };
                let names: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
                for k in h.keys() {
                    let k = show(k);
                    if !names.contains(&k.as_str()) {
                        return Err(format!(
                            "`{path}` has an unknown field `{k}`. The record's fields are: {} (case-sensitive).{}",
                            names.join(", "),
                            did_you_mean(&k, names.iter().copied())
                        ));
                    }
                }
                let mut out = Vec::new();
                for (n, ft) in fields {
                    let fy = h
                        .get(&Yaml::String(n.clone()))
                        .ok_or_else(|| format!("`{path}` is missing the field `{n}`."))?;
                    out.push((n.clone(), self.conv(fy, ft, &format!("{path}.{n}"))?));
                }
                Ok(Value::Rec(out))
            }
        }
    }
}

/// Parse one test case. Returns the environment with every variable the test defines.
pub fn load(spec: &Spec, text: &str) -> Result<Env, Diagnostic> {
    let docs = YamlLoader::load_from_str(text).map_err(|e| {
        let m = e.marker();
        Diagnostic::new(m.line().max(1), m.col() + 1, "The test data is not valid")
            .explain(format!("The data reader says: {}.", e.info()))
            .hint("Each line is `name: value`; arrays look like `[1, 2, 3]`, records like `{name: AS, time: 37}`.")
            .data()
    })?;
    let empty = yaml_rust2::yaml::Hash::new();
    let map = match docs.first() {
        None | Some(Yaml::Null) => &empty,
        Some(Yaml::Hash(h)) => h,
        Some(_) => {
            return Err(Diagnostic::new(
                1,
                1,
                "The test data must be a list of `name: value` lines",
            )
            .hint("For example:\nn: 3\nnums: [1, 2, 3]")
            .data());
        }
    };
    let declared: Vec<&str> = spec.decls.iter().map(|d| d.name.as_str()).collect();
    for k in map.keys() {
        let k = show(k);
        if !declared.contains(&k.as_str()) {
            let line = key_line(text, &k);
            let sugg = did_you_mean(&k, declared.iter().copied());
            return Err(Diagnostic::new(line, 1, format!("`{k}` is not declared in the specification"))
                .explain(format!("Test data keys must match the declared variable names exactly (case-sensitive).{sugg}"))
                .hint("Fix the name here, or declare the variable in the specification.")
                .data());
        }
    }
    let mut env = Env::new();
    let mut ev = Eval::new(spec);
    // scalars first, so array bounds like [1..n] can use them
    let is_arr = |t: &Type| matches!(spec.resolve(t), Some(Type::Array { .. }));
    let order = spec
        .decls
        .iter()
        .filter(|d| !is_arr(&d.ty))
        .chain(spec.decls.iter().filter(|d| is_arr(&d.ty)));
    for d in order {
        let line = key_line(text, &d.name);
        let value = match map.get(&Yaml::String(d.name.clone())) {
            Some(y) => {
                let mut c = Conv {
                    spec,
                    ev: &mut ev,
                    env: &mut env,
                };
                c.conv(y, &d.ty, &d.name).map_err(|msg| {
                    Diagnostic::new(
                        line,
                        1,
                        format!(
                            "`{}: {}` doesn't match the declaration `{}∈{}`",
                            d.name,
                            show(y),
                            d.name,
                            type_name(spec, &d.ty)
                        ),
                    )
                    .explain(msg)
                    .data()
                })?
            }
            None => match &d.init {
                Some(init) => {
                    let mut v = ev.eval(init, &mut env.clone())?;
                    if let (Value::Arr { from, .. }, Some(Type::Array { lo, .. })) =
                        (&mut v, spec.resolve(&d.ty))
                        && let Ok(Value::Int(f)) = ev.eval(lo, &mut env.clone())
                    {
                        *from = f;
                    }
                    v
                }
                None if d.role == Role::In => {
                    return Err(Diagnostic::plain(format!(
                        "Input `{}` is missing from this test case",
                        d.name
                    ))
                    .explain("Every input variable needs a value in each test case.")
                    .hint(format!(
                        "Add a line `{}: …`, or press Scaffold to insert every variable name.",
                        d.name
                    )));
                }
                None => continue, // aux/out without expected value: the postcondition may compute it
            },
        };
        env.insert(d.name.clone(), value);
    }
    Ok(env)
}
