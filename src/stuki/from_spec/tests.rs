use super::*;
use crate::parser::parse;

/// The structograms as indented text: `[while i=1..n]`, `[if c]` … `[else]`, `[case c]`.
fn outline(p: &Program) -> String {
    fn block(nodes: &[Node], depth: usize, out: &mut String) {
        let pad = "  ".repeat(depth);
        for n in nodes {
            match &n.kind {
                Kind::Noop => out.push_str(&format!("{pad}-\n")),
                Kind::Statement(t) => {
                    let mark = if n.bg.is_some() { " (!)" } else { "" };
                    out.push_str(&format!("{pad}{}{mark}\n", t.text));
                }
                Kind::If { cond, yes, no } => {
                    out.push_str(&format!("{pad}[if {}]\n", cond.text));
                    block(yes, depth + 1, out);
                    if !(no.len() == 1 && no[0].is_noop()) {
                        out.push_str(&format!("{pad}[else]\n"));
                        block(no, depth + 1, out);
                    }
                }
                Kind::While { cond, body } => {
                    out.push_str(&format!("{pad}[while {}]\n", cond.text));
                    block(body, depth + 1, out);
                }
                Kind::Repeat { cond, body } => {
                    out.push_str(&format!("{pad}[repeat {}]\n", cond.text));
                    block(body, depth + 1, out);
                }
                Kind::Switch { cases } => {
                    for c in cases {
                        let cond = c.cond.as_ref().map_or("else", |c| &c.text);
                        out.push_str(&format!("{pad}[case {cond}]\n"));
                        block(&c.body, depth + 1, out);
                    }
                }
            }
        }
    }
    let mut s = String::new();
    for d in &p.diagrams {
        s.push_str(&format!("== {}\n", d.name.text));
        if d.show_variables {
            s.push_str(&format!("vars {}\n", d.variables.text));
        }
        block(&d.body, 0, &mut s);
        if d.show_header && d.show_return {
            s.push_str(&format!("return {}\n", d.ret.text));
        }
    }
    s
}

fn outline_of(spec: &str) -> String {
    let spec = parse(spec).unwrap_or_else(|e| panic!("{}", e[0].title));
    outline(&from_spec(&spec, "T").program)
}

#[test]
fn count() {
    let out = outline_of(
        "Be:\nn∈N,\nszamok∈N[1..n]\nKi:\ndb∈N\nEf:\n-\nUf:\ndb=DARAB(i=1..n, szamok[i] mod 2 = 1)\n",
    );
    assert_eq!(
        out,
        "== T\nvars i:Egész\ndb:=0\n[while i=1..n]\n  [if szamok[i] mod 2 = 1]\n    db:=db+1\nKi: db\n"
    );
}

#[test]
fn sum_with_condition_and_english_words() {
    let out = outline_of(
        "In:\nn∈N,\nx∈R[1..n]\nOut:\ns∈R\nPre:\n-\nPost:\ns=SUM(i=1..n, x[i], x[i]>0)\n",
    );
    assert_eq!(
        out,
        "== T\nvars i:Integer\ns:=0\n[while i=1..n]\n  [if x[i]>0]\n    s:=s+x[i]\nOut: s\n"
    );
}

#[test]
fn max_and_search() {
    let out = outline_of(
        "Be:\nn∈N,\nx∈Z[1..n]\nKi:\nmi∈N,\nme∈Z,\nvan∈L,\nind∈N\nEf:\nn>0\nUf:\n(mi,me)=MAX(i=1..n, x[i]) és (van,ind)=KERES(j=1..n, x[j]<0)\n",
    );
    assert_eq!(
        out,
        "== T\nvars i:Egész, j:Egész\n\
         mi:=1\nme:=x[1]\n[while i=2..n]\n  [if x[i]>me]\n    mi:=i\n    me:=x[i]\n\
         j:=1\n[while j≤n és nem (x[j]<0)]\n  j:=j+1\nvan:=j≤n\n[if van]\n  ind:=j\nKi: mi, me, van, ind\n"
    );
}

#[test]
fn empty_slot_gets_a_local_and_order_follows_dependencies() {
    let out = outline_of(
        "Be:\nn∈N,\nx∈Z[1..n]\nKi:\nme∈Z,\nk∈Z\nEf:\nn>0\nUf:\nk=me*2 és (,me)=MAX(i=1..n, x[i])\n",
    );
    assert!(
        out.starts_with("== T\nvars maxind:Egész, i:Egész\nmaxind:=1\nme:=x[1]\n"),
        "{out}"
    );
    assert!(out.ends_with("k:=me*2\nKi: me, k\n"), "{out}");
}

#[test]
fn nested_pattern_and_elementwise_loop() {
    let out = outline_of(
        "Be:\nn∈N,\nm∈N,\nt∈Z[1..n,1..m]\nKi:\ny∈N[1..n]\nEf:\n-\nUf:\n∀i∈[1..n]:(y[i]=DARAB(j=1..m, t[i,j]>0))\n",
    );
    assert_eq!(
        out,
        "== T\nvars i:Egész, db:Egész, j:Egész\n[while i=1..n]\n  db:=0\n  [while j=1..m]\n    [if t[i,j]>0]\n      db:=db+1\n  y[i]:=db\nKi: y\n"
    );
}

#[test]
fn functions_get_their_own_structogram() {
    let out = outline_of(
        "Be:\nn∈N,\nx∈Z[1..n]\nKi:\nvan∈L\nFv:\njo:N->L, jo(i)=x[i]>0 és x[i] mod 2=0\nEf:\n-\nUf:\nvan=∃i∈[1..n]:(jo(i))\n",
    );
    assert_eq!(
        out,
        "== T\nvars i:Egész\ni:=1\n[while i≤n és nem jo(i)]\n  i:=i+1\nvan:=i≤n\nKi: van\n\
         == jo(i:Egész):Logikai\n-\nreturn x[i]>0 és x[i] mod 2=0\n"
    );
}

#[test]
fn condmax_and_a_condition_on_the_result() {
    let spec = parse(
        "Be:\nn∈N,\nx∈Z[1..n]\nKi:\nvan∈L,\nmi∈N,\nme∈Z\nEf:\n-\nUf:\n(van,mi,me)=MAXHA(i=1..n, x[i], x[i]<0) és me<0\n",
    )
    .unwrap();
    let g = from_spec(&spec, "");
    assert!(g.unset.is_empty());
    let out = outline(&g.program);
    assert_eq!(
        out,
        "== Főprogram\nvars i:Egész\nvan:=hamis\n[while i=1..n]\n\
         [case nem (x[i]<0)]\n    -\n\
         [case x[i]<0 és van]\n    [if x[i]>me]\n      mi:=i\n      me:=x[i]\n\
         [case x[i]<0 és nem van]\n    van:=igaz\n    mi:=i\n    me:=x[i]\n\
         Ki: van, mi, me\n"
            .replace("[case", "  [case")
    );
}

#[test]
fn implications_become_one_decision() {
    let out = outline_of(
        "Be:\nn∈N,\nar∈N[1..n],\nk∈N\nSa:\nvan∈L,\nind∈N\nKi:\nsorszam∈Z\nEf:\n-\nUf:\n\
         (van -> sorszam=ind) és (nem van -> sorszam=-1) és\n(van,ind)=KERES(i=1..n, ar[i]<k) és k=k\n",
    );
    assert_eq!(
        out,
        "== T\nvars i:Egész\ni:=1\n[while i≤n és nem (ar[i]<k)]\n  i:=i+1\nvan:=i≤n\n\
         [if van]\n  ind:=i\n[if van]\n  sorszam:=ind\n[else]\n  sorszam:=-1\nKi: sorszam\n"
    );
    // a search inside the condition comes first; the rest of the condition decides
    let out = outline_of(
        "In:\nn∈N,\nx∈N[1..n]\nAux:\nvan∈L,\nind∈N\nOut:\nlib∈N\nPre:\n-\nPost:\n\
         (van,ind)=SEARCH(i=1..n-1, x[i]<x[i+1]) and\n      van -> lib=x[ind]\n",
    );
    assert!(
        out.ends_with("[if van]\n  ind:=i\n[if van]\n  lib:=x[ind]\nOut: lib\n"),
        "{out}"
    );
}

#[test]
fn names_are_unique_per_structogram() {
    let out = outline_of(
        "In:\nn∈N,\nm∈N,\nt∈N[1..n,1..m]\nOut:\nmx∈N\nFn:\nmc:N->N x N, mc(i)=MAX(j=1..m, t[i,j])\nPre:\n-\nPost:\n(mx,)=MAX(i=1..n, mc(i).maxval)\n",
    );
    assert!(
        out.contains("== mc(i:Integer):Integer x Integer\nvars maxind:Integer, maxval:Integer"),
        "{out}"
    );
    assert!(out.contains("maxval:=mc(1).maxval\n"), "{out}");
}

#[test]
fn sum_in_a_function() {
    let out = outline_of(
        "Be:\nn∈N,\nx∈R[1..n]\nKi:\natl∈R\nFv:\nossz:N->R, ossz(k)=SZUMMA(i=1..k, x[i])\nEf:\nn>0\nUf:\natl=ossz(n)/n\n",
    );
    assert_eq!(
        out,
        "== T\natl:=ossz(n)/n\nKi: atl\n\
         == ossz(k:Egész):Valós\nvars s:Valós, i:Egész\ns:=0\n[while i=1..k]\n  s:=s+x[i]\nreturn s\n"
    );
}

const TABLE: &str = "Be:\nn∈Z,\nm∈Z,\nx∈Z[1..n],\nt∈Z[1..n,1..m]\nKi:\n";
const REST: &str = " és n=n és m=m és x=x és t=t\n";

fn table(tail: &str) -> String {
    outline_of(&format!("{TABLE}{tail}{REST}"))
}

#[test]
fn branches_that_exclude_each_other_are_one_decision() {
    assert_eq!(
        table("s∈Z\nEf:\n-\nUf:\n(n>m -> s=n) és (n≤m -> s=m)"),
        "== T\n[if n>m]\n  s:=n\n[else]\n  s:=m\nKi: s\n"
    );
    assert_eq!(
        table("s∈Z\nEf:\n-\nUf:\n(n<m -> s=n) és (m≤n -> s=m)"),
        "== T\n[if n<m]\n  s:=n\n[else]\n  s:=m\nKi: s\n"
    );
    assert_eq!(
        table("s∈Z\nEf:\n-\nUf:\n(n>0 -> s=1) és (n=0 -> s=0) és (n<0 -> s=-1)"),
        "== T\n[case n>0]\n  s:=1\n[case n=0]\n  s:=0\n[case n<0]\n  s:=-1\nKi: s\n"
    );
}

#[test]
fn a_nested_loop_with_the_same_variable_gets_its_own() {
    assert_eq!(
        table("s∈Z\nEf:\n-\nUf:\ns=SZUMMA(i=1..n, SZUMMA(i=1..m, t[1,i]))"),
        "== T\nvars i:Egész, s2:Egész, i2:Egész\ns:=0\n[while i=1..n]\n  s2:=0\n  \
         [while i2=1..m]\n    s2:=s2+t[1,i2]\n  s:=s+s2\nKi: s\n"
    );
}

#[test]
fn patterns_inside_a_condition_become_steps() {
    assert_eq!(
        table("van∈L\nEf:\n-\nUf:\nvan=∃i∈[1..n]:(SZUMMA(j=1..m, t[i,j])>0)"),
        "== T\nvars i:Egész, s:Egész, j:Egész\nvan:=hamis\ni:=1\n\
         [while i≤n és nem van]\n  s:=0\n  [while j=1..m]\n    s:=s+t[i,j]\n  van:=s>0\n  i:=i+1\nKi: van\n"
    );
    let search = table("van∈L,\nk∈N\nEf:\n-\nUf:\n(van,k)=KERES(i=1..n, SZUMMA(j=1..m, t[i,j])>3)");
    assert!(
        search.ends_with("  van:=s>3\n  i:=i+1\n[if van]\n  k:=i-1\nKi: van, k\n"),
        "{search}"
    );
    let all = table("van∈L\nEf:\n-\nUf:\nvan=∀i∈[1..n]:(DARAB(j=1..m, t[i,j]>0)>0)");
    assert!(
        all.contains("van:=igaz\ni:=1\n[while i≤n és van]\n"),
        "{all}"
    );
    let select = table("k∈N\nEf:\n-\nUf:\nk=KIVÁLASZT(i>=1, SZUMMA(j=1..m, t[i,j])>0)");
    assert!(
        select.contains("van:=hamis\ni:=0\n[while nem van]\n  i:=i+1\n  s:=0\n")
            && select.ends_with("  van:=s>0\nk:=i\nKi: k\n"),
        "{select}"
    );
    let condmax = table(
        "van∈L,\nmi∈N,\nme∈Z\nEf:\n-\nUf:\n(van,mi,me)=MAXHA(i=1..n, x[i], DARAB(j=1..m, t[i,j]>0)>2)",
    );
    assert!(
        condmax.contains("  [case nem (db>2)]\n") && condmax.contains("  [case db>2 és van]\n"),
        "{condmax}"
    );
    let decision = table(
        "s∈Z\nEf:\n-\nUf:\n(DARAB(i=1..n, x[i]>0)>1 -> s=1) és (DARAB(i=1..n, x[i]>0)≤1 -> s=0)",
    );
    assert!(
        decision.ends_with("[if db>1]\n  s:=1\n[else]\n  s:=0\nKi: s\n"),
        "{decision}"
    );
    for out in [search, all, select, condmax, decision] {
        assert!(!out.contains("SZUMMA") && !out.contains("DARAB"), "{out}");
    }
}

#[test]
fn the_first_element_of_a_maximum_gets_the_same_steps() {
    assert_eq!(
        table("mi∈N,\nme∈Z\nEf:\nn>0\nUf:\n(mi,me)=MAX(i=1..n, SZUMMA(j=1..m, t[i,j]))"),
        "== T\nvars i:Egész, s:Egész, j:Egész\n[while i=1..n]\n  s:=0\n  [while j=1..m]\n    \
         s:=s+t[i,j]\n  [if i=1 vagy s>me]\n    mi:=i\n    me:=s\nKi: mi, me\n"
    );
}

#[test]
fn a_part_of_a_pattern_becomes_steps() {
    assert_eq!(
        table("s∈Z\nEf:\nn>0\nUf:\ns=MAX(i=1..n, x[i]).maxval+1"),
        "== T\nvars maxind:Egész, maxért:Egész, i:Egész\nmaxind:=1\nmaxért:=x[1]\n\
         [while i=2..n]\n  [if x[i]>maxért]\n    maxind:=i\n    maxért:=x[i]\ns:=maxért+1\nKi: s\n"
    );
    // issue #5: the maximum inside the condition of a filter
    let out = outline_of(
        "Be: nDb∈N, tDb∈N, h∈Z[1..tDb,1..nDb]\nKi: db∈N, maxosak∈N[1..db]\nEf: 1<=nDb\n\
         Uf: (db, maxosak)=KIVÁLOGAT(i=1..tDb,∃j∈[1..nDb]:(h[i,j] = MAX(i=1..tDb,h[i,j]).2),i)\n",
    );
    assert!(!out.contains("(!)") && !out.contains("MAX"), "{out}");
}

#[test]
fn elements_and_loops_with_decisions_become_steps() {
    assert_eq!(
        table("y∈Z[1..n]\nEf:\n-\nUf:\ny[1]=x[1] és ∀i∈[2..n]:(y[i]=y[i-1]+x[i])"),
        "== T\nvars i:Egész\ny[1]:=x[1]\n[while i=2..n]\n  y[i]:=y[i-1]+x[i]\nKi: y\n"
    );
    assert_eq!(
        table("y∈Z[1..n]\nEf:\n-\nUf:\n∀i∈[1..n]:((x[i]>0 -> y[i]=1) és (x[i]≤0 -> y[i]=0))"),
        "== T\nvars i:Egész\n[while i=1..n]\n  [if x[i]>0]\n    y[i]:=1\n  [else]\n    y[i]:=0\nKi: y\n"
    );
    // conditions on the result are left out
    assert_eq!(
        table(
            "db∈N,\ny∈Z[1..db]\nEf:\n-\nUf:\n(db,y)=KIVÁLOGAT(i=1..n, x[i]>0, x[i]) és ∀i∈[1..db]:(y[i]>0)"
        ),
        "== T\nvars i:Egész\ndb:=0\n[while i=1..n]\n  [if x[i]>0]\n    db:=db+1\n    y[db]:=x[i]\nKi: db, y\n"
    );
}

#[test]
fn the_else_branch_may_set_what_the_pattern_set() {
    // issue #9
    let out = outline_of(
        "Be:\nn∈N, x∈Z[1..n]\nSa:\nvan∈L\nKi:\nm∈Z\nEf:\n-\nUf:\n\
         van -> (van, , m)=FELTMAX(i=1..n, x[i], x[i]>0) és\nnem van -> m = 0\n",
    );
    assert!(
        out.ends_with("[if van]\n  -\n[else]\n  m:=0\nKi: m\n"),
        "{out}"
    );
}

#[test]
fn a_pattern_that_cannot_become_steps_leaves_its_output_unset() {
    let spec = parse(&format!(
        "{TABLE}s∈Z\nEf:\nn>0\nUf:\ns=MAX(i=1..n, x[i]).3{REST}"
    ))
    .unwrap();
    let g = from_spec(&spec, "T");
    assert_eq!(g.unset, ["s"]);
    assert_eq!(outline(&g.program), "== T\nKi: s\n");
}
