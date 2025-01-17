use fast_hash::FxHashMap;
use std::fmt;
use syntax::ast::SyntaxNodePtr;
use syntax::rowan::TextRange;

/// Pointers between the AST and the HIR.
#[derive(Debug, Default)]
pub struct Ptrs {
  hir_to_ast: FxHashMap<hir::Idx, SyntaxNodePtr>,
  ast_to_hir: FxHashMap<SyntaxNodePtr, hir::Idx>,
}

impl Ptrs {
  /// Returns the unique syntax pointer for an HIR index.
  pub fn hir_to_ast(&self, idx: hir::Idx) -> Option<SyntaxNodePtr> {
    self.hir_to_ast.get(&idx).cloned()
  }

  /// Returns one of possibly many HIR indices for the syntax pointer.
  pub fn ast_to_hir(&self, ptr: SyntaxNodePtr) -> Option<hir::Idx> {
    self.ast_to_hir.get(&ptr).copied()
  }

  fn insert(&mut self, idx: hir::Idx, ptr: SyntaxNodePtr) {
    assert!(self.hir_to_ast.insert(idx, ptr.clone()).is_none());
    // cannot assert is none
    self.ast_to_hir.insert(ptr, idx);
  }
}

/// An error when lowering.
#[derive(Debug)]
pub struct Error {
  range: TextRange,
  kind: ErrorKind,
}

impl Error {
  /// Returns the range for this.
  pub fn range(&self) -> TextRange {
    self.range
  }

  /// Returns a value that displays the message.
  pub fn display(&self) -> impl fmt::Display + '_ {
    &self.kind
  }

  /// Returns the code for this.
  pub fn to_code(&self) -> u16 {
    match self.kind {
      ErrorKind::FunBindMismatchedName(_, _) => 4001,
      ErrorKind::FunBindWrongNumPats(_, _) => 4002,
      ErrorKind::InvalidIntLit(_) | ErrorKind::InvalidBigIntLit(_) => 4003,
      ErrorKind::InvalidRealLit(_) => 4004,
      ErrorKind::InvalidNumLab(_) | ErrorKind::ZeroNumLab => 4005,
      ErrorKind::MultipleRestPatRows => 4006,
      ErrorKind::RestPatRowNotLast => 4007,
      ErrorKind::PrecedingBar => 4008,
      ErrorKind::RequiresOperand => 4009,
      ErrorKind::Unsupported(_) => 4999,
    }
  }
}

#[derive(Debug)]
pub(crate) enum ErrorKind {
  FunBindMismatchedName(String, String),
  FunBindWrongNumPats(usize, usize),
  InvalidIntLit(std::num::ParseIntError),
  InvalidBigIntLit(hir::ParseBigIntError),
  InvalidRealLit(std::num::ParseFloatError),
  InvalidNumLab(std::num::ParseIntError),
  ZeroNumLab,
  MultipleRestPatRows,
  RestPatRowNotLast,
  PrecedingBar,
  RequiresOperand,
  /// must be last
  Unsupported(&'static str),
}

impl fmt::Display for ErrorKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ErrorKind::FunBindMismatchedName(want, got) => {
        write!(
          f,
          "expected a function clause for {want}, found one for {got}"
        )
      }
      ErrorKind::FunBindWrongNumPats(want, got) => {
        write!(f, "expected {want} patterns, found {got}")
      }
      ErrorKind::InvalidIntLit(e) => write!(f, "invalid literal: {e}"),
      ErrorKind::InvalidBigIntLit(e) => write!(f, "invalid literal: {e}"),
      ErrorKind::InvalidRealLit(e) => write!(f, "invalid literal: {e}"),
      ErrorKind::InvalidNumLab(e) => write!(f, "invalid numeric label: {e}"),
      ErrorKind::ZeroNumLab => f.write_str("invalid numeric label: numeric labels start at 1"),
      ErrorKind::MultipleRestPatRows => f.write_str("multiple `...`"),
      ErrorKind::RestPatRowNotLast => f.write_str("`...` must come last"),
      ErrorKind::PrecedingBar => f.write_str("preceding `|`"),
      ErrorKind::RequiresOperand => f.write_str("requires at least 1 operand"),
      ErrorKind::Unsupported(s) => write!(f, "unsupported language construct: {s}"),
    }
  }
}

/// The result of lowering.
#[derive(Debug)]
pub struct Lower {
  /// The errors.
  pub errors: Vec<Error>,
  /// The arenas.
  pub arenas: hir::Arenas,
  /// The pointers.
  pub ptrs: Ptrs,
  /// The single top declaration. Often a sequence of decs.
  pub root: hir::StrDecIdx,
}

#[derive(Debug, Default)]
pub(crate) struct Cx {
  fresh_idx: u32,
  errors: Vec<Error>,
  arenas: hir::Arenas,
  ptrs: Ptrs,
}

impl Cx {
  /// Returns a `Name` that is both:
  /// - not writeable in user code, and will thus not collide with any identifiers in user code;
  /// - distinct from all other `Name`s returned from self thus far, and will thus not collide
  ///   with any of those.
  pub(crate) fn fresh(&mut self) -> hir::Name {
    let ret = hir::Name::new(self.fresh_idx.to_string());
    self.fresh_idx += 1;
    ret
  }

  pub(crate) fn err(&mut self, range: TextRange, kind: ErrorKind) {
    self.errors.push(Error { range, kind })
  }

  pub(crate) fn finish(self, root: hir::StrDecIdx) -> Lower {
    Lower {
      errors: self.errors,
      arenas: self.arenas,
      ptrs: self.ptrs,
      root,
    }
  }

  pub(crate) fn str_dec(&mut self, val: hir::StrDec, ptr: SyntaxNodePtr) -> hir::StrDecIdx {
    let idx = self.arenas.str_dec.alloc(val);
    self.ptrs.insert(idx.into(), ptr);
    Some(idx)
  }

  pub(crate) fn str_exp(&mut self, val: hir::StrExp, ptr: SyntaxNodePtr) -> hir::StrExpIdx {
    let idx = self.arenas.str_exp.alloc(val);
    self.ptrs.insert(idx.into(), ptr);
    Some(idx)
  }

  pub(crate) fn sig_exp(&mut self, val: hir::SigExp, ptr: SyntaxNodePtr) -> hir::SigExpIdx {
    let idx = self.arenas.sig_exp.alloc(val);
    self.ptrs.insert(idx.into(), ptr);
    Some(idx)
  }

  pub(crate) fn spec(&mut self, val: hir::Spec, ptr: SyntaxNodePtr) -> hir::SpecIdx {
    let idx = self.arenas.spec.alloc(val);
    self.ptrs.insert(idx.into(), ptr);
    Some(idx)
  }

  pub(crate) fn exp(&mut self, val: hir::Exp, ptr: SyntaxNodePtr) -> hir::ExpIdx {
    let idx = self.arenas.exp.alloc(val);
    self.ptrs.insert(idx.into(), ptr);
    Some(idx)
  }

  pub(crate) fn dec(&mut self, val: hir::Dec, ptr: SyntaxNodePtr) -> hir::DecIdx {
    let idx = self.arenas.dec.alloc(val);
    self.ptrs.insert(idx.into(), ptr);
    Some(idx)
  }

  pub(crate) fn pat(&mut self, val: hir::Pat, ptr: SyntaxNodePtr) -> hir::PatIdx {
    let idx = self.arenas.pat.alloc(val);
    self.ptrs.insert(idx.into(), ptr);
    Some(idx)
  }

  pub(crate) fn ty(&mut self, val: hir::Ty, ptr: SyntaxNodePtr) -> hir::TyIdx {
    let idx = self.arenas.ty.alloc(val);
    self.ptrs.insert(idx.into(), ptr);
    Some(idx)
  }
}
