use crate::common::{get_lab, get_path, get_scon};
use crate::util::{Cx, ErrorKind};
use crate::{dec, pat, ty};
use syntax::ast::{self, AstNode as _, SyntaxNodePtr};

pub(crate) fn get(cx: &mut Cx, exp: Option<ast::Exp>) -> hir::ExpIdx {
  let exp = exp?;
  let ptr = SyntaxNodePtr::new(exp.syntax());
  let ret = match exp {
    ast::Exp::HoleExp(_) | ast::Exp::WildcardExp(_) => hir::Exp::Hole,
    ast::Exp::SConExp(exp) => hir::Exp::SCon(get_scon(cx, exp.s_con()?)?),
    ast::Exp::PathExp(exp) => hir::Exp::Path(get_path(exp.path()?)?),
    ast::Exp::RecordExp(exp) => hir::Exp::Record(
      exp
        .exp_rows()
        .filter_map(|row| {
          let lab_ast = row.lab()?;
          let lab_tr = lab_ast.token.text_range();
          let lab = get_lab(cx, lab_ast);
          let exp = match row.eq_exp() {
            Some(eq_exp) => get(cx, eq_exp.exp()),
            None => match &lab {
              hir::Lab::Name(name) => {
                cx.err(lab_tr, ErrorKind::Unsupported("expression row punning"));
                cx.exp(hir::Exp::Path(hir::Path::one(name.clone())), ptr.clone())
              }
              hir::Lab::Num(_) => {
                // NOTE: we explicitly duplicate the `err` call in both branches, to remind us that
                // if we ever actually accepted expression row punning, we should add a separate
                // error here rejecting the attempt to pun with a int label.
                cx.err(lab_tr, ErrorKind::Unsupported("expression row punning"));
                None
              }
            },
          };
          Some((lab, exp))
        })
        .collect(),
    ),
    ast::Exp::SelectorExp(exp) => {
      let lab = get_lab(cx, exp.lab()?);
      let fresh = cx.fresh();
      let pat = cx.pat(pat::name(fresh.as_str()), ptr.clone());
      let param = cx.pat(
        hir::Pat::Record {
          rows: vec![(lab, pat)],
          allows_other: true,
        },
        ptr.clone(),
      );
      let body = cx.exp(name(fresh.as_str()), ptr.clone());
      hir::Exp::Fn(vec![(param, body)])
    }
    // sml_def(5)
    ast::Exp::ParenExp(exp) => return get(cx, exp.exp()),
    ast::Exp::TupleExp(exp) => tuple(exp.exp_args().map(|e| get(cx, e.exp()))),
    ast::Exp::ListExp(exp) => {
      // need to rev()
      #[allow(clippy::needless_collect)]
      let exps: Vec<_> = exp.exp_args().map(|x| get(cx, x.exp())).collect();
      exps.into_iter().rev().fold(name("nil"), |ac, x| {
        let cons = cx.exp(name("::"), ptr.clone());
        let ac = cx.exp(ac, ptr.clone());
        hir::Exp::App(cons, cx.exp(tuple([x, ac]), ptr.clone()))
      })
    }
    ast::Exp::VectorExp(exp) => {
      cx.err(
        exp.syntax().text_range(),
        ErrorKind::Unsupported("vector expressions"),
      );
      return None;
    }
    ast::Exp::SeqExp(exp) => return exps_in_seq(cx, exp.exps_in_seq(), ptr),
    ast::Exp::LetExp(exp) => {
      let dec = dec::get(cx, exp.dec());
      let exp = exps_in_seq(cx, exp.exps_in_seq(), ptr.clone());
      hir::Exp::Let(dec, exp)
    }
    ast::Exp::AppExp(exp) => hir::Exp::App(get(cx, exp.func()), get(cx, exp.arg())),
    ast::Exp::InfixExp(exp) => {
      let func = exp
        .name_star_eq()
        .and_then(|x| cx.exp(name(x.token.text()), ptr.clone()));
      let lhs = get(cx, exp.lhs());
      let rhs = get(cx, exp.rhs());
      let arg = cx.exp(tuple([lhs, rhs]), ptr.clone());
      hir::Exp::App(func, arg)
    }
    ast::Exp::TypedExp(exp) => hir::Exp::Typed(get(cx, exp.exp()), ty::get(cx, exp.ty())),
    ast::Exp::AndalsoExp(exp) => {
      let cond = get(cx, exp.lhs());
      let yes = get(cx, exp.rhs());
      let no = cx.exp(name("false"), ptr.clone());
      if_(cx, cond, yes, no, ptr.clone())
    }
    ast::Exp::OrelseExp(exp) => {
      let cond = get(cx, exp.lhs());
      let yes = cx.exp(name("true"), ptr.clone());
      let no = get(cx, exp.rhs());
      if_(cx, cond, yes, no, ptr.clone())
    }
    ast::Exp::HandleExp(exp) => hir::Exp::Handle(get(cx, exp.exp()), matcher(cx, exp.matcher())),
    ast::Exp::RaiseExp(exp) => hir::Exp::Raise(get(cx, exp.exp())),
    ast::Exp::IfExp(exp) => {
      let cond = get(cx, exp.cond());
      let yes = get(cx, exp.yes());
      let no = get(cx, exp.no());
      if_(cx, cond, yes, no, ptr.clone())
    }
    ast::Exp::WhileExp(exp) => {
      let vid = cx.fresh();
      let fn_body = {
        let cond = get(cx, exp.cond());
        let body = get(cx, exp.body());
        let call = call_unit_fn(cx, &vid, ptr.clone());
        let yes = exp_idx_in_seq(cx, [body, call], ptr.clone());
        let no = cx.exp(tuple([]), ptr.clone());
        let fn_body = if_(cx, cond, yes, no, ptr.clone());
        cx.exp(fn_body, ptr.clone())
      };
      let arg_pat = cx.pat(pat::tuple([]), ptr.clone());
      let fn_exp = cx.exp(hir::Exp::Fn(vec![(arg_pat, fn_body)]), ptr.clone());
      let vid_pat = cx.pat(pat::name(vid.as_str()), ptr.clone());
      let val = cx.dec(
        hir::Dec::Val(
          vec![],
          vec![hir::ValBind {
            rec: true,
            pat: vid_pat,
            exp: fn_exp,
          }],
        ),
        ptr.clone(),
      );
      hir::Exp::Let(val, call_unit_fn(cx, &vid, ptr.clone()))
    }
    ast::Exp::CaseExp(exp) => {
      let head = get(cx, exp.exp());
      let arms = matcher(cx, exp.matcher());
      case(cx, head, arms, ptr.clone())
    }
    ast::Exp::FnExp(exp) => hir::Exp::Fn(matcher(cx, exp.matcher())),
  };
  cx.exp(ret, ptr)
}

pub(crate) fn name(s: &str) -> hir::Exp {
  hir::Exp::Path(hir::Path::one(hir::Name::new(s)))
}

pub(crate) fn tuple<I>(es: I) -> hir::Exp
where
  I: IntoIterator<Item = hir::ExpIdx>,
{
  let rows: Vec<_> = es
    .into_iter()
    .enumerate()
    .map(|(idx, e)| (hir::Lab::tuple(idx), e))
    .collect();
  assert_ne!(rows.len(), 1);
  hir::Exp::Record(rows)
}

fn call_unit_fn(cx: &mut Cx, vid: &hir::Name, ptr: SyntaxNodePtr) -> hir::ExpIdx {
  let vid_exp = cx.exp(name(vid.as_str()), ptr.clone());
  let arg_exp = cx.exp(hir::Exp::Record(vec![]), ptr.clone());
  cx.exp(hir::Exp::App(vid_exp, arg_exp), ptr)
}

fn exps_in_seq<I>(cx: &mut Cx, exps: I, ptr: SyntaxNodePtr) -> hir::ExpIdx
where
  I: Iterator<Item = ast::ExpInSeq>,
{
  let exps: Vec<_> = exps.into_iter().map(|e| get(cx, e.exp())).collect();
  exp_idx_in_seq(cx, exps, ptr)
}

/// lowers 1 into 2. (which is then lowered into 3.)
///
/// 1. `(e1; ...; en; e)`
/// 2. `(case e1 of _ => ... => case en of _ => e)`
/// 3. `((fn _ => ... (fn _ => (fn _ => e) en) ...) e1)`
///
/// the vec must not be empty, since we need a last expression `e`.
fn exp_idx_in_seq<A, B>(cx: &mut Cx, exps: A, ptr: SyntaxNodePtr) -> hir::ExpIdx
where
  A: IntoIterator<IntoIter = B>,
  B: DoubleEndedIterator<Item = hir::ExpIdx>,
{
  exps
    .into_iter()
    .rev()
    .reduce(|ac, x| {
      let wild = cx.pat(hir::Pat::Wild, ptr.clone());
      let c = case(cx, x, vec![(wild, ac)], ptr.clone());
      cx.exp(c, ptr.clone())
    })
    .flatten()
}

fn if_(
  cx: &mut Cx,
  cond: hir::ExpIdx,
  yes: hir::ExpIdx,
  no: hir::ExpIdx,
  ptr: SyntaxNodePtr,
) -> hir::Exp {
  let yes_pat = cx.pat(pat::name("true"), ptr.clone());
  let no_pat = cx.pat(pat::name("false"), ptr.clone());
  case(cx, cond, vec![(yes_pat, yes), (no_pat, no)], ptr)
}

pub(crate) fn case(
  cx: &mut Cx,
  head: hir::ExpIdx,
  arms: Vec<(hir::PatIdx, hir::ExpIdx)>,
  ptr: SyntaxNodePtr,
) -> hir::Exp {
  hir::Exp::App(cx.exp(hir::Exp::Fn(arms), ptr), head)
}

fn matcher(cx: &mut Cx, matcher: Option<ast::Matcher>) -> Vec<(hir::PatIdx, hir::ExpIdx)> {
  if let Some(bar) = matcher.as_ref().and_then(|m| m.bar()) {
    cx.err(bar.text_range(), ErrorKind::PrecedingBar);
  }
  matcher
    .into_iter()
    .flat_map(|x| x.match_rules())
    .map(|arm| (pat::get(cx, arm.pat()), get(cx, arm.exp())))
    .collect()
}
