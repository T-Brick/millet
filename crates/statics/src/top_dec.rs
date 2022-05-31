#![allow(dead_code)]

use crate::dec;
use crate::error::{ErrorKind, Item};
use crate::st::St;
use crate::ty;
use crate::types::{
  generalize, Bs, Cx, Env, FixedTyVars, IdStatus, Sig, StrEnv, TyEnv, TyScheme, ValInfo,
};
use crate::util::{cannot_bind_val, get_env, get_ty_info, ins_no_dupe};

pub(crate) fn get(st: &mut St, bs: &mut Bs, ars: &hir::Arenas, top_dec: hir::TopDecIdx) {
  match &ars.top_dec[top_dec] {
    // sml_def(87)
    hir::TopDec::Str(str_dec) => {
      let mut env = Env::default();
      get_str_dec(st, bs, ars, &mut env, *str_dec);
      bs.env.extend(env);
    }
    // sml_def(66), sml_def(88)
    hir::TopDec::Sig(sig_binds) => {
      // sml_def(67)
      for _ in sig_binds {
        st.err(top_dec, ErrorKind::Unsupported)
      }
    }
    // sml_def(85), sml_def(89)
    hir::TopDec::Functor(fun_binds) => {
      // sml_def(86)
      for _ in fun_binds {
        st.err(top_dec, ErrorKind::Unsupported)
      }
    }
  }
}

fn get_str_exp(st: &mut St, bs: &Bs, ars: &hir::Arenas, env: &mut Env, str_exp: hir::StrExpIdx) {
  let str_exp = match str_exp {
    Some(x) => x,
    None => return,
  };
  match &ars.str_exp[str_exp] {
    // sml_def(50)
    hir::StrExp::Struct(str_dec) => get_str_dec(st, bs, ars, env, *str_dec),
    // sml_def(51)
    hir::StrExp::Path(path) => match get_env(&bs.env, path.all_names()) {
      Ok(got_env) => env.extend(got_env.clone()),
      Err(name) => st.err(str_exp, ErrorKind::Undefined(Item::Struct, name.clone())),
    },
    // sml_def(52), sml_def(53)
    hir::StrExp::Ascription(_, _, _) => st.err(str_exp, ErrorKind::Unsupported),
    // sml_def(54)
    hir::StrExp::App(_, _) => st.err(str_exp, ErrorKind::Unsupported),
    // sml_def(55)
    hir::StrExp::Let(str_dec, str_exp) => {
      let mut let_env = Env::default();
      get_str_dec(st, bs, ars, &mut let_env, *str_dec);
      let mut bs = bs.clone();
      bs.env.extend(let_env);
      get_str_exp(st, &bs, ars, env, *str_exp)
    }
  }
}

fn get_str_dec(st: &mut St, bs: &Bs, ars: &hir::Arenas, env: &mut Env, str_dec: hir::StrDecIdx) {
  let str_dec = match str_dec {
    Some(x) => x,
    None => return,
  };
  match &ars.str_dec[str_dec] {
    // sml_def(56)
    hir::StrDec::Dec(dec) => dec::get(st, &bs.as_cx(), ars, env, *dec),
    // sml_def(57)
    hir::StrDec::Structure(str_binds) => {
      // sml_def(61)
      let mut str_env = StrEnv::default();
      for str_bind in str_binds {
        let mut env = Env::default();
        get_str_exp(st, bs, ars, &mut env, str_bind.str_exp);
        if let Some(e) = ins_no_dupe(&mut str_env, str_bind.name.clone(), env, Item::Struct) {
          st.err(str_dec, e);
        }
      }
      env.str_env.extend(str_env);
    }
    // sml_def(58)
    hir::StrDec::Local(local_dec, in_dec) => {
      let mut local_env = Env::default();
      get_str_dec(st, bs, ars, &mut local_env, *local_dec);
      let mut bs = bs.clone();
      bs.env.extend(local_env);
      get_str_dec(st, &bs, ars, env, *in_dec);
    }
    // sml_def(59), sml_def(60)
    hir::StrDec::Seq(str_decs) => {
      let mut bs = bs.clone();
      for &str_dec in str_decs {
        let mut one_env = Env::default();
        get_str_dec(st, &bs, ars, &mut one_env, str_dec);
        bs.env.extend(one_env.clone());
        env.extend(one_env);
      }
    }
  }
}

fn get_sig_exp(st: &mut St, bs: &Bs, ars: &hir::Arenas, env: &mut Env, sig_exp: hir::SigExpIdx) {
  let sig_exp = match sig_exp {
    Some(x) => x,
    None => return,
  };
  match &ars.sig_exp[sig_exp] {
    // sml_def(62)
    hir::SigExp::Spec(spec) => get_spec(st, bs, ars, env, *spec),
    // sml_def(63)
    hir::SigExp::Name(_) => st.err(sig_exp, ErrorKind::Unsupported),
    // sml_def(64)
    hir::SigExp::Where(_, _, _, _) => st.err(sig_exp, ErrorKind::Unsupported),
  }
}

// sml_def(65)
fn env_to_sig(_: &mut St, _: &Bs, _: Env) -> Sig {
  todo!()
}

fn get_spec(st: &mut St, bs: &Bs, ars: &hir::Arenas, env: &mut Env, spec: hir::SpecIdx) {
  let spec = match spec {
    Some(x) => x,
    None => return,
  };
  match &ars.spec[spec] {
    // sml_def(68)
    hir::Spec::Val(val_descs) => {
      // sml_def(79)
      let cx = bs.as_cx();
      for val_desc in val_descs {
        let mut ty_scheme = TyScheme::zero(ty::get(st, &cx, ars, val_desc.ty));
        generalize(st.subst(), FixedTyVars::default(), &mut ty_scheme);
        let vi = ValInfo {
          ty_scheme,
          id_status: IdStatus::Val,
        };
        let name = &val_desc.name;
        if cannot_bind_val(name.as_str()) {
          st.err(spec, ErrorKind::InvalidRebindName(name.clone()));
        } else if let Some(e) = ins_no_dupe(&mut env.val_env, name.clone(), vi, Item::Val) {
          st.err(spec, e);
        }
      }
    }
    // sml_def(69)
    hir::Spec::Ty(_) => st.err(spec, ErrorKind::Unsupported),
    // sml_def(70)
    hir::Spec::EqTy(_) => st.err(spec, ErrorKind::Unsupported),
    // sml_def(71)
    hir::Spec::Datatype(dat_descs) => {
      let (ty_env, big_val_env) = dec::get_dat_binds(st, bs.as_cx(), ars, dat_descs, spec.into());
      for (name, val) in ty_env {
        if let Some(e) = ins_no_dupe(&mut env.ty_env, name, val, Item::Ty) {
          st.err(spec, e);
        }
      }
      for (name, val) in big_val_env {
        if let Some(e) = ins_no_dupe(&mut env.val_env, name, val, Item::Val) {
          st.err(spec, e);
        }
      }
    }
    // sml_def(72)
    hir::Spec::DatatypeCopy(name, path) => match get_ty_info(&bs.env, path) {
      Ok(ty_info) => {
        if let Some(e) = ins_no_dupe(&mut env.ty_env, name.clone(), ty_info.clone(), Item::Ty) {
          st.err(spec, e);
        }
      }
      Err(e) => st.err(spec, e),
    },
    // sml_def(73)
    hir::Spec::Exception(ex_descs) => {
      // sml_def(83)
      for _ in ex_descs {
        st.err(spec, ErrorKind::Unsupported)
      }
    }
    // sml_def(74)
    hir::Spec::Str(str_descs) => {
      // sml_def(84)
      for _ in str_descs {
        st.err(spec, ErrorKind::Unsupported)
      }
    }
    // sml_def(75)
    hir::Spec::Include(sig_exp) => get_sig_exp(st, bs, ars, env, *sig_exp),
    // sml_def(78)
    hir::Spec::Sharing(_, _) => st.err(spec, ErrorKind::Unsupported),
    // sml_def(76), sml_def(77)
    hir::Spec::Seq(specs) => {
      for &spec in specs {
        get_spec(st, bs, ars, env, spec);
      }
    }
  }
}

// sml_def(80)
fn get_ty_descs(_: &mut St, _: &Cx, _: &hir::Arenas, _: &mut TyEnv, _: &[hir::TyDesc]) {
  todo!()
}
