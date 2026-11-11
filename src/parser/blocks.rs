//! Splitting a specification into its `In:` / `Pre:` / `Post:` … blocks, and checking their order.

use super::ast::line_col;
use crate::diag::{Diagnostic, suggest};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BlockKind {
    In,
    Aux,
    Out,
    Fn,
    Pre,
    Post,
}

const HEADERS: &[(&str, BlockKind)] = &[
    ("In", BlockKind::In),
    ("Be", BlockKind::In),
    ("Aux", BlockKind::Aux),
    ("Sa", BlockKind::Aux),
    ("Out", BlockKind::Out),
    ("Ki", BlockKind::Out),
    ("Fn", BlockKind::Fn),
    ("Fv", BlockKind::Fn),
    ("Pre", BlockKind::Pre),
    ("Ef", BlockKind::Pre),
    ("Post", BlockKind::Post),
    ("Uf", BlockKind::Post),
    ("Def", BlockKind::Fn),
];

/// Plausible but wrong header spellings → the real one.
const WRONG_HEADERS: &[(&str, &str)] = &[
    ("segéd", "Sa:"),
    ("seged", "Sa:"),
    ("segedvaltozo", "Sa:"),
    ("bemenet", "Be:"),
    ("kimenet", "Ki:"),
    ("függvény", "Fv:"),
    ("fuggveny", "Fv:"),
    ("előfeltétel", "Ef:"),
    ("elofeltetel", "Ef:"),
    ("utófeltétel", "Uf:"),
    ("utofeltetel", "Uf:"),
    ("input", "In:"),
    ("output", "Out:"),
    ("auxiliary", "Aux:"),
    ("function", "Fn:"),
    ("functions", "Fn:"),
    ("precondition", "Pre:"),
    ("postcondition", "Post:"),
];

fn kind_name(k: BlockKind, hu: bool) -> &'static str {
    let i = HEADERS.iter().position(|(_, kk)| *kk == k).unwrap();
    HEADERS[i + hu as usize].0
}

pub(super) struct Block {
    pub kind: BlockKind,
    pub kw: String,
    line: usize,
    pub body: (usize, usize),
}

pub(super) fn split_blocks(src: &str) -> Result<Vec<Block>, Diagnostic> {
    let all_headers = "In: Be: Aux: Sa: Out: Ki: Fn: Fv: Def: Pre: Ef: Post: Uf:";
    let mut blocks: Vec<Block> = Vec::new();
    let mut offset = 0;
    let mut first_content: Option<usize> = None;
    for (idx, line) in src.split('\n').enumerate() {
        let line_no = idx + 1;
        let lstart = offset;
        offset += line.len() + 1;
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let word: String = trimmed
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let after = &trimmed[word.len()..];
        let col = line[..indent].chars().count() + 1;
        let colon_follows = after.trim_start_matches([' ', '\t']).starts_with(':')
            && !after.trim_start().starts_with(":=");
        let exact = HEADERS.iter().find(|(h, _)| *h == word);
        if let Some((h, kind)) =
            exact.filter(|_| after.starts_with(':') && !after.starts_with(":="))
        {
            let rest = &after[1..];
            if rest.chars().next().is_some_and(|c| !c.is_whitespace()) {
                return Err(Diagnostic::new(line_no, col + word.chars().count() + 1, format!("Missing space after `{h}:`"))
                    .doc("ch02-01-blocks.html").explain("A block header is followed either by a space (the content continues on the same line) or by a line break (the content starts on the next line). The ELTE editor rejects anything else.")
                    .hint(format!("Write `{h}: {}`", rest.trim())));
            }
            let body_start = lstart + indent + word.len() + 1;
            if let Some(last) = blocks.last_mut() {
                last.body.1 = lstart;
            } else if let Some(fc) = first_content {
                let (l, c) = line_col(src, fc);
                return Err(Diagnostic::new(l, c, "Text before the first block")
                    .doc("ch02-01-blocks.html").explain("A specification must start with the input block. Only comments may come before it.")
                    .hint("Start with `In:` (or `Be:`), or turn this text into a comment with `//`."));
            }
            blocks.push(Block {
                kind: *kind,
                kw: format!("{h}:"),
                line: line_no,
                body: (body_start, src.len()),
            });
            continue;
        }
        if !word.is_empty() && colon_follows {
            let lower = word.to_lowercase();
            let wrong = WRONG_HEADERS
                .iter()
                .find(|(w, _)| *w == lower)
                .map(|(_, r)| *r);
            let case_twin = HEADERS
                .iter()
                .find(|(h, _)| h.to_lowercase() == lower)
                .map(|(h, _)| *h);
            let looks_like_header = word.chars().next().unwrap().is_uppercase()
                || wrong.is_some()
                || case_twin.is_some();
            if looks_like_header {
                let mut d =
                    Diagnostic::new(line_no, col, format!("Unknown block header `{word}:`"));
                if let Some(h) = case_twin.filter(|h| *h == word) {
                    d = d
                        .doc("ch02-01-blocks.html")
                        .explain(format!(
                            "There must be no space between `{h}` and the colon."
                        ))
                        .hint(format!("Write `{h}:`"));
                } else if let Some(h) = case_twin {
                    d = d
                        .doc("ch02-01-blocks.html")
                        .explain(format!(
                            "Block headers are case-sensitive. The valid headers are: {all_headers}"
                        ))
                        .hint(format!("Write `{h}:`"));
                } else if let Some(r) = wrong {
                    d = d
                        .doc("ch02-01-blocks.html").explain(format!("`{word}:` looks like a translation of a header, but the editor only knows these: {all_headers}"))
                        .hint(format!("Write `{r}`"));
                } else {
                    let s = suggest(&word, HEADERS.iter().map(|(h, _)| *h));
                    d = d
                        .doc("ch02-01-blocks.html")
                        .explain(format!("The valid block headers are: {all_headers}"));
                    if let Some(s) = s {
                        d = d.hint(format!("Did you mean `{s}:`?"));
                    }
                }
                return Err(d);
            }
        }
        if blocks.is_empty() && first_content.is_none() && !trimmed.trim().is_empty() {
            first_content = Some(lstart + indent);
        }
    }
    if blocks.is_empty() {
        if let Some(fc) = first_content {
            let (l, c) = line_col(src, fc);
            return Err(Diagnostic::new(l, c, "No block headers found")
                .doc("ch02-01-blocks.html")
                .explain(format!(
                    "A specification consists of blocks that start with a header: {all_headers}"
                ))
                .hint("Start with `In: n∈N` (or `Be:` on its own line)."));
        }
        return Err(Diagnostic::plain("The specification is empty")
            .doc("ch02-01-blocks.html")
            .hint("Start with an `In:` / `Be:` block."));
    }

    // order and presence
    let hu = blocks[0].kw == "Be:"
        || blocks
            .iter()
            .any(|b| matches!(b.kw.as_str(), "Be:" | "Ki:" | "Ef:" | "Uf:"));
    let rank = |k: BlockKind| match k {
        BlockKind::In | BlockKind::Aux | BlockKind::Out => 0,
        BlockKind::Fn => 1,
        BlockKind::Pre => 2,
        BlockKind::Post => 3,
    };
    let mut seen: Vec<&Block> = Vec::new();
    for b in &blocks {
        // functions: one `Fn:` header per function, so that block repeats
        if let Some(prev) = seen
            .iter()
            .find(|p| p.kind == b.kind && b.kind != BlockKind::Fn)
        {
            return Err(Diagnostic::new(b.line, 1, format!("Second `{}` block", b.kw))
                .doc("ch02-01-blocks.html").explain(format!("This kind of block was already started at line {} (`{}`). Each block may appear only once.", prev.line, prev.kw))
                .hint("Merge the two blocks: separate the items with commas."));
        }
        if let Some(later) = seen.iter().find(|p| rank(p.kind) > rank(b.kind)) {
            let what = if rank(b.kind) == 0 {
                "Declarations (In/Aux/Out) come first, then Fn, then Pre, then Post."
            } else {
                "The order is: declarations (In/Aux/Out), then Fn, then Pre, then Post."
            };
            return Err(
                Diagnostic::new(b.line, 1, format!("`{}` is in the wrong place", b.kw))
                    .doc("ch02-01-blocks.html#block-order")
                    .explain(format!(
                        "It comes after `{}` (line {}). {what}",
                        later.kw, later.line
                    ))
                    .hint(format!("Move the `{}` block before `{}`.", b.kw, later.kw)),
            );
        }
        seen.push(b);
    }
    for (k, why) in [
        (BlockKind::In, "it declares the input variables"),
        (BlockKind::Out, "it declares the output variables"),
        (
            BlockKind::Pre,
            "write `- ` as its content if there is no precondition",
        ),
        (BlockKind::Post, "it states what the program must achieve"),
    ] {
        if !blocks.iter().any(|b| b.kind == k) {
            let name = kind_name(k, hu);
            let mut d = Diagnostic::plain(format!("Missing `{name}:` block"))
                .doc("ch02-01-blocks.html")
                .explain(format!("This block is compulsory: {why}."));
            if k == BlockKind::Pre {
                d = d.hint(format!("Add `{name}: -`"));
            }
            return Err(d);
        }
    }
    Ok(blocks)
}
