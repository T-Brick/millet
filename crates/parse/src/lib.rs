//! Parses tokens into a concrete syntax tree.

#![deny(missing_debug_implementations, missing_docs, rust_2018_idioms)]

mod dec;
mod exp;
mod pat;
mod root;
mod top_dec;
mod ty;
mod util;

pub mod parser;

use syntax::ast::AstNode as _;
use syntax::{token::Token, SyntaxKind as SK};

pub use syntax::ast::Root;

/// The result of a parse.
#[derive(Debug)]
pub struct Parse {
  /// The root.
  pub root: Root,
  /// The errors encountered when parsing.
  pub errors: Vec<parser::Error>,
}

/// Returns a parse of the tokens.
pub fn get<'a>(tokens: &'a [Token<'a, SK>], fix_env: &'a mut parser::FixEnv) -> Parse {
  let mut p = parser::Parser::new(tokens, fix_env);
  root::root(&mut p);
  let (node, errors) = p.finish();
  Parse {
    root: Root::cast(node).unwrap(),
    errors,
  }
}
