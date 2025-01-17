//! Text of various SML libraries.

#![deny(missing_debug_implementations, missing_docs, rust_2018_idioms)]

pub mod primitive;
pub mod sml_nj;
pub mod std_basis;
pub mod std_basis_extra;

macro_rules! files {
  ( $( $x:literal ),* $(,)? ) => {{
    &[
      $(
        ($x, include_str!($x)),
      )*
    ]
  }};
}

pub(crate) use files;
