//! Lexer, block splitter, parser and static checks for specifications.

mod ast;
mod blocks;
mod check;
mod expr;
mod grammar;
mod lexer;

pub use lexer::closing_quotes;

pub use ast::{
    BUILTINS, BinOp, Decl, Expr, ExprKind, Func, MACROS, MAX_DEPTH, Role, Spec, Type, line_col,
    macro_alias, vars_in,
};

use crate::diag::Diagnostic;
use blocks::BlockKind;
use grammar::Parser;
use lexer::Tok;
use std::collections::HashMap;

pub fn parse(src: &str) -> Result<Spec, Vec<Diagnostic>> {
    let spec = parse_blocks(src).map_err(|d| vec![d])?;
    let diags = check::check(&spec);
    if diags.is_empty() {
        Ok(spec)
    } else {
        Err(diags)
    }
}

fn parse_blocks(src: &str) -> Result<Spec, Diagnostic> {
    let clean = lexer::strip_comments(src)?;
    let blocks = blocks::split_blocks(&clean)?;
    let mut decls = Vec::new();
    let mut aliases = HashMap::new();
    let mut funcs = HashMap::new();
    let (mut pre, mut post) = (None, None);
    for b in &blocks {
        let toks = lexer::lex(&clean, src, b.body.0, b.body.1)?;
        let mut p = Parser::new(src, toks, &b.kw);
        let parsed = match b.kind {
            BlockKind::In => p.decl_block(Role::In, &mut decls, &mut aliases),
            BlockKind::Aux => p.decl_block(Role::Aux, &mut decls, &mut aliases),
            BlockKind::Out => p.decl_block(Role::Out, &mut decls, &mut aliases),
            BlockKind::Fn => p.fn_block(&mut funcs),
            BlockKind::Pre => p
                .condition_block(true)
                .map(|e| pre = Some((e, b.kw.clone()))),
            BlockKind::Post => p
                .condition_block(false)
                .map(|e| post = Some((e, b.kw.clone()))),
        };
        if let Some(d) = p.depth_error() {
            return Err(d);
        }
        parsed?;
    }
    let (pre, pre_kw) = pre.expect("a Pre block");
    let (post, post_kw) = post.expect("a Post block");
    Ok(Spec {
        src: src.to_string(),
        decls,
        aliases,
        funcs,
        pre,
        post,
        pre_kw,
        post_kw,
    })
}

#[derive(Clone, Debug)]
pub enum Fragment {
    Assign(Expr, Expr),
    Range(String, Expr, Expr),
    Expr(Expr),
}

pub fn parse_fragment(src: &str, from: usize, to: usize) -> Result<Fragment, Diagnostic> {
    let toks = lexer::lex(src, src, from, to)?;
    let counting = matches!(toks.first().map(|t| &t.tok), Some(Tok::Ident(_)))
        && toks.get(1).is_some_and(|t| t.tok == Tok::Sym("="))
        && {
            let mut depth = 0;
            toks.iter().any(|t| {
                match t.tok {
                    Tok::Sym("(" | "[") => depth += 1,
                    Tok::Sym(")" | "]") => depth -= 1,
                    _ => {}
                }
                depth == 0 && t.tok == Tok::Sym("..")
            })
        };
    let assigns = toks.iter().any(|t| t.tok == Tok::Sym(":="));
    let mut p = Parser::new(src, toks, "structogram");
    let parsed = if assigns {
        let target = p.add()?;
        p.expect(":=", "`:=`")?;
        Fragment::Assign(target, p.expr()?)
    } else if counting {
        let var = p.ident("loop variable")?;
        p.expect("=", "`=`")?;
        let lo = p.add()?;
        p.expect("..", "`..`")?;
        Fragment::Range(var, lo, p.add()?)
    } else {
        Fragment::Expr(p.expr()?)
    };
    if let Some(d) = p.depth_error() {
        return Err(d);
    }
    if p.peek().tok != Tok::Eof {
        return Err(p.unexpected("the end of the text"));
    }
    Ok(parsed)
}
