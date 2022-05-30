use crate::util::{Cx, ErrorKind};
use syntax::ast;

/// unfortunately, although we already kind of "parsed" these tokens in lex, that information is not
/// carried to here. so we must do it again.
pub(crate) fn get_scon(cx: &mut Cx, scon: ast::SCon) -> Option<hir::SCon> {
  let tok = scon.token;
  let ret = match scon.kind {
    ast::SConKind::IntLit => {
      let chars = tok.text();
      let (mul, chars) = start_int(chars, "0x");
      let n = match i32::from_str_radix(chars.as_str(), 16) {
        Ok(x) => mul * x,
        Err(e) => {
          cx.err(tok.text_range(), ErrorKind::InvalidIntLit(e));
          0
        }
      };
      hir::SCon::Int(n)
    }
    ast::SConKind::RealLit => {
      let owned: String;
      let mut text = tok.text();
      // only alloc if needed
      if text.contains('~') {
        owned = tok.text().replace('~', "-");
        text = owned.as_str();
      }
      let n = match text.parse() {
        Ok(x) => x,
        Err(e) => {
          cx.err(tok.text_range(), ErrorKind::InvalidRealLit(e));
          0.0
        }
      };
      hir::SCon::Real(n)
    }
    ast::SConKind::WordLit => {
      let chars = tok.text();
      let (mul, mut chars) = start_int(chars, "0w");
      if chars.as_str().starts_with('x') {
        chars.next();
      }
      let n = match i32::from_str_radix(chars.as_str(), 16) {
        Ok(x) => mul * x,
        Err(e) => {
          cx.err(tok.text_range(), ErrorKind::InvalidIntLit(e));
          0
        }
      };
      hir::SCon::Word(n)
    }
    ast::SConKind::CharLit => hir::SCon::Char(tok.text().bytes().next()?),
    ast::SConKind::StringLit => hir::SCon::String(tok.text().into()),
  };
  Some(ret)
}

fn start_int<'a>(chars: &'a str, prefix: &str) -> (i32, std::str::Chars<'a>) {
  let mut chars = chars.chars();
  let neg = chars.as_str().starts_with('~');
  if neg {
    chars.next();
  }
  if chars.as_str().starts_with(prefix) {
    chars.next();
    chars.next();
  }
  (if neg { -1 } else { 1 }, chars)
}

pub(crate) fn get_name(n: Option<syntax::SyntaxToken>) -> Option<hir::Name> {
  n.map(|tok| hir::Name::new(tok.text()))
}

pub(crate) fn get_path(p: ast::Path) -> Option<hir::Path> {
  hir::Path::try_new(
    p.name_plus_dots()
      .filter_map(|x| Some(hir::Name::new(x.name_plus()?.token.text())))
      .collect(),
  )
}

pub(crate) fn get_lab(lab: ast::Lab) -> Option<hir::Lab> {
  match lab.kind {
    ast::LabKind::Name | ast::LabKind::Star => {
      Some(hir::Lab::Name(hir::Name::new(lab.token.text())))
    }
    ast::LabKind::IntLit => lab.token.text().parse::<usize>().ok().map(hir::Lab::Num),
  }
}
