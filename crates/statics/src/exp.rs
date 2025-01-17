use crate::error::{ErrorKind, Item};
use crate::get_env::get_val_info;
use crate::info::TyEntry;
use crate::pat_match::Pat;
use crate::st::St;
use crate::types::{
  Cx, Def, Env, EnvLike as _, Generalizable, Sym, SymsMarker, Ty, TyScheme, ValEnv,
};
use crate::unify::unify;
use crate::util::{apply, get_scon, instantiate, record};
use crate::{dec, pat, ty};

pub(crate) fn get(st: &mut St, cx: &Cx, ars: &hir::Arenas, exp: hir::ExpIdx) -> Ty {
  let exp = match exp {
    Some(x) => x,
    None => return Ty::None,
  };
  // NOTE: do not early return, since we add to the Info at the bottom.
  let mut ty_scheme = None::<TyScheme>;
  let mut def = None::<Def>;
  let ret = match &ars.exp[exp] {
    hir::Exp::Hole => {
      let mv = st.meta_gen.gen(Generalizable::Always);
      st.insert_hole(mv, exp.into());
      Ty::MetaVar(mv)
    }
    // sml_def(1)
    hir::Exp::SCon(scon) => get_scon(st, scon, Generalizable::Always),
    // sml_def(2)
    hir::Exp::Path(path) => match get_val_info(&cx.env, path) {
      Ok(Some(val_info)) => {
        ty_scheme = Some(val_info.ty_scheme.clone());
        def = val_info.def;
        instantiate(st, val_info.ty_scheme.clone(), Generalizable::Always)
      }
      Ok(None) => {
        st.err(exp, ErrorKind::Undefined(Item::Val, path.last().clone()));
        Ty::None
      }
      Err(e) => {
        st.err(exp, e);
        Ty::None
      }
    },
    // sml_def(3)
    hir::Exp::Record(rows) => {
      let rows = record(st, rows, exp.into(), |st, _, exp| get(st, cx, ars, exp));
      Ty::Record(rows)
    }
    // sml_def(4)
    hir::Exp::Let(dec, inner) => {
      let mut let_env = Env::default();
      let marker = st.syms.mark();
      dec::get(st, cx, ars, &mut let_env, *dec);
      let mut cx = cx.clone();
      cx.env.append(&mut let_env);
      let got = get(st, &cx, ars, *inner);
      if let Some(sym) = ty_name_escape(&marker, &got) {
        st.err(inner.unwrap_or(exp), ErrorKind::TyNameEscape(sym));
      }
      got
    }
    // sml_def(8)
    hir::Exp::App(func, arg) => {
      let func_ty = get(st, cx, ars, *func);
      let arg_ty = get(st, cx, ars, *arg);
      // we could choose to not `match` on `func_ty` and just use the `MetaVar` case always and it
      // would still be correct. however, matching on `func_ty` lets us emit slightly better error
      // messages sometimes.
      match func_ty {
        Ty::None => Ty::None,
        Ty::BoundVar(_) => unreachable!("bound vars should be instantiated"),
        Ty::MetaVar(_) => {
          let mut ret = Ty::MetaVar(st.meta_gen.gen(Generalizable::Always));
          let got = Ty::fun(arg_ty, ret.clone());
          unify(st, func_ty, got, exp.into());
          apply(st.subst(), &mut ret);
          ret
        }
        Ty::FixedVar(_) | Ty::Record(_) | Ty::Con(_, _) => {
          st.err(func.unwrap_or(exp), ErrorKind::AppLhsNotFn(func_ty));
          Ty::None
        }
        Ty::Fn(want_arg, mut want_res) => {
          unify(st, *want_arg, arg_ty, arg.unwrap_or(exp).into());
          apply(st.subst(), want_res.as_mut());
          *want_res
        }
      }
    }
    // sml_def(10)
    hir::Exp::Handle(inner, matcher) => {
      let mut exp_ty = get(st, cx, ars, *inner);
      let (pats, param, res) = get_matcher(st, cx, ars, matcher, exp.into());
      let idx = inner.unwrap_or(exp);
      unify(st, Ty::EXN, param.clone(), idx.into());
      unify(st, exp_ty.clone(), res, idx.into());
      apply(st.subst(), &mut exp_ty);
      st.insert_handle(pats, param, idx.into());
      exp_ty
    }
    // sml_def(11)
    hir::Exp::Raise(inner) => {
      let got = get(st, cx, ars, *inner);
      unify(st, Ty::EXN, got, inner.unwrap_or(exp).into());
      Ty::MetaVar(st.meta_gen.gen(Generalizable::Always))
    }
    // sml_def(12)
    hir::Exp::Fn(matcher) => {
      let (pats, param, res) = get_matcher(st, cx, ars, matcher, exp.into());
      st.insert_case(pats, param.clone(), exp.into());
      Ty::fun(param, res)
    }
    // sml_def(9)
    hir::Exp::Typed(inner, want) => {
      let got = get(st, cx, ars, *inner);
      let mut want = ty::get(st, cx, ars, *want);
      unify(st, want.clone(), got, exp.into());
      apply(st.subst(), &mut want);
      want
    }
  };
  let ty_entry = TyEntry {
    ty: ret.clone(),
    ty_scheme,
  };
  st.info().insert(exp.into(), Some(ty_entry), def);
  ret
}

/// sml_def(13)
fn get_matcher(
  st: &mut St,
  cx: &Cx,
  ars: &hir::Arenas,
  matcher: &[(hir::PatIdx, hir::ExpIdx)],
  idx: hir::Idx,
) -> (Vec<Pat>, Ty, Ty) {
  let mut param_ty = Ty::MetaVar(st.meta_gen.gen(Generalizable::Always));
  let mut res_ty = Ty::MetaVar(st.meta_gen.gen(Generalizable::Always));
  let mut pats = Vec::<Pat>::new();
  st.meta_gen.inc_rank();
  // sml_def(14)
  for &(pat, exp) in matcher {
    let mut ve = ValEnv::default();
    let (pm_pat, pat_ty) = pat::get(st, cx, ars, &mut ve, pat, Generalizable::Sometimes);
    let mut cx = cx.clone();
    cx.env.push(Env {
      val_env: ve,
      ..Default::default()
    });
    let exp_ty = get(st, &cx, ars, exp);
    let pi = pat.map_or(idx, Into::into);
    unify(st, param_ty.clone(), pat_ty, pi);
    let ei = exp.map_or(idx, Into::into);
    unify(st, res_ty.clone(), exp_ty, ei);
    apply(st.subst(), &mut param_ty);
    apply(st.subst(), &mut res_ty);
    pats.push(pm_pat);
  }
  st.meta_gen.dec_rank();
  (pats, param_ty, res_ty)
}

fn ty_name_escape(m: &SymsMarker, ty: &Ty) -> Option<Sym> {
  match ty {
    Ty::None | Ty::BoundVar(_) | Ty::MetaVar(_) | Ty::FixedVar(_) => None,
    Ty::Record(rows) => rows.values().find_map(|ty| ty_name_escape(m, ty)),
    Ty::Con(args, sym) => sym
      .generated_after(m)
      .then_some(*sym)
      .or_else(|| args.iter().find_map(|ty| ty_name_escape(m, ty))),
    Ty::Fn(param, res) => ty_name_escape(m, param).or_else(|| ty_name_escape(m, res)),
  }
}
