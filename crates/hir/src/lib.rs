//! High-level Intermediate Representation for SML.

#![deny(missing_debug_implementations)]
#![deny(rust_2018_idioms)]

use la_arena::{Arena, Idx};
use smol_str::SmolStr;
use std::fmt;

pub use la_arena;

pub type ExpIdx = Idx<Exp>;
pub type ExpArena = Arena<Exp>;

#[derive(Debug)]
pub enum Exp {
  None,
  SCon(SCon),
  Path(Path),
  Record(Vec<(Lab, ExpIdx)>),
  Seq(Vec<ExpIdx>),
  Let(DecIdx, ExpIdx),
  App(ExpIdx, ExpIdx),
  Handle(ExpIdx, Vec<(PatIdx, ExpIdx)>),
  Raise(ExpIdx),
  Fn(Vec<(PatIdx, ExpIdx)>),
}

pub type DecIdx = Idx<Dec>;
pub type DecArena = Arena<Dec>;

#[derive(Debug)]
pub enum Dec {
  None,
  Val(Vec<TyVar>, Vec<ValBind>),
  Ty(Vec<TyBind>),
  Datatype(Vec<DatBind>),
}

#[derive(Debug)]
pub struct ValBind {
  pub rec: bool,
  pub pat: PatIdx,
  pub exp: ExpIdx,
}

#[derive(Debug)]
pub struct TyBind {
  pub ty_vars: Vec<TyVar>,
  pub name: Name,
  pub ty: TyIdx,
}

#[derive(Debug)]
pub struct DatBind {
  pub ty_vars: Vec<TyVar>,
  pub name: Name,
  pub cons: Vec<(Name, Option<TyIdx>)>,
}

pub type PatIdx = Idx<Pat>;
pub type PatArena = Arena<Pat>;

#[derive(Debug)]
pub enum Pat {
  None,
}

pub type TyIdx = Idx<Ty>;
pub type TyArena = Arena<Ty>;

#[derive(Debug)]
pub enum Ty {
  None,
}

#[derive(Debug)]
pub enum Lab {
  Num(usize),
  Name(Name),
}

#[derive(Debug)]
pub enum SCon {
  Int,
  Real,
  Word,
  Char,
  String,
}

#[derive(Debug)]
pub struct Path(Vec<Name>);

impl Path {
  pub fn new(names: Vec<Name>) -> Option<Self> {
    assert!(!names.is_empty());
    Some(Self(names))
  }

  pub fn last(&self) -> &Name {
    self.0.last().unwrap()
  }
}

#[derive(Debug)]
pub struct Name(SmolStr);

impl Name {
  pub fn new(s: &str) -> Self {
    assert!(!s.is_empty());
    Self(s.into())
  }

  #[inline(always)]
  pub fn as_str(&self) -> &str {
    self.0.as_str()
  }
}

impl fmt::Display for Name {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

#[derive(Debug)]
pub struct TyVar(SmolStr);

impl TyVar {
  pub fn new(s: &str) -> Self {
    assert!(s.len() >= 2);
    assert!(s.as_bytes()[0] == b'\'');
    Self(s.into())
  }

  pub fn is_equality(&self) -> bool {
    self.0.as_bytes()[1] == b'\''
  }

  #[inline(always)]
  pub fn as_str(&self) -> &str {
    self.0.as_str()
  }
}

impl fmt::Display for TyVar {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}