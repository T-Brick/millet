//! Analysis.

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(rust_2018_idioms)]

use syntax::{ast::AstNode as _, rowan::TextRange};

type SyntaxNodePtr = syntax::ast::SyntaxNodePtr<syntax::SML>;

/// An error.
#[derive(Debug)]
pub struct Error {
  /// The range of the error.
  pub range: TextRange,
  /// The message of the error.
  pub message: String,
}

/// Returns a Vec of Vec of errors for each file.
///
/// The length of the returned Vec will be the same as the length of files.
///
/// TODO should probably have this be more like a data structure that can support adding/removing
/// files and other queries other than errors. Then we could be more incremental.
pub fn get<'a, I>(files: I) -> Vec<Vec<Error>>
where
  I: Iterator<Item = &'a str>,
{
  let mut files: Vec<_> = files
    .map(|contents| {
      let lexed = lex::get(contents);
      let parsed = parse::get(&lexed.tokens);
      let lowered = lower::get(&parsed.root);
      AnalyzedFile {
        lex_errors: lexed.errors,
        parsed,
        lowered,
        statics_errors: Vec::new(),
      }
    })
    .collect();
  let mut st = statics::Statics::default();
  for file in files.iter_mut() {
    statics::get(&mut st, &file.lowered.arenas, &file.lowered.top_decs);
    file.statics_errors = std::mem::take(&mut st.errors);
  }
  files
    .into_iter()
    .map(|file| {
      std::iter::empty()
        .chain(file.lex_errors.into_iter().map(|err| Error {
          range: err.range,
          message: err.kind.to_string(),
        }))
        .chain(file.parsed.errors.into_iter().map(|err| Error {
          range: err.range,
          message: err.kind.to_string(),
        }))
        .chain(file.lowered.errors.into_iter().map(|err| Error {
          range: err.range,
          message: err.kind.to_string(),
        }))
        .chain(file.statics_errors.into_iter().filter_map(|err| {
          Some(Error {
            range: get_statics_ptr(&file.lowered.ptrs, err.idx())?
              .to_node(file.parsed.root.syntax())
              .text_range(),
            message: err.display(&st.syms).to_string(),
          })
        }))
        .collect()
    })
    .collect()
}

fn get_statics_ptr(ptrs: &lower::Ptrs, idx: statics::Idx) -> Option<SyntaxNodePtr> {
  match idx {
    statics::Idx::Exp(idx) => ptrs.get_exp(idx),
    statics::Idx::Pat(idx) => ptrs.get_pat(idx),
    statics::Idx::Ty(idx) => ptrs.get_ty(idx),
    statics::Idx::Dec(idx) => ptrs.get_dec(idx),
    statics::Idx::StrDec(idx) => ptrs.get_str_dec(idx),
    statics::Idx::TopDec(idx) => ptrs.get_top_dec(idx),
  }
}

struct AnalyzedFile {
  lex_errors: Vec<lex::Error>,
  parsed: parse::Parse,
  lowered: lower::Lower,
  statics_errors: Vec<statics::Error>,
}