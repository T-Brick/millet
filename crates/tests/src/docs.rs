use std::path::{Path, PathBuf};

use crate::check::{check_with_std_basis, fail_with_std_basis};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag};

const SML: &str = "sml";

fn check_all<F>(path: &Path, mut f: F)
where
  F: FnMut(&str),
{
  let contents = std::fs::read_to_string(&path).unwrap();
  let mut options = Options::empty();
  options.insert(Options::ENABLE_TABLES);
  let parser = Parser::new_ext(&contents, options);
  let mut inside = false;
  let mut ac = String::new();
  for ev in parser {
    match ev {
      Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
        if lang.as_ref() == SML {
          inside = true;
        }
      }
      Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
        if lang.as_ref() == SML {
          f(ac.as_str());
          ac.clear();
          inside = false;
        }
      }
      Event::Text(s) => {
        if inside {
          ac.push_str(s.as_ref());
        }
      }
      _ => {}
    }
  }
}

fn docs_dir() -> Option<PathBuf> {
  Some(
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .parent()?
      .parent()?
      .join("docs"),
  )
}

#[test]
fn errors() {
  let path = docs_dir().unwrap().join("errors.md");
  check_all(path.as_path(), |s| {
    if s.starts_with("(* ok *)") {
      check_with_std_basis(s);
    } else if s.starts_with("(* error *)") {
      fail_with_std_basis(s);
    }
  });
}

#[test]
fn keywords() {
  let path = docs_dir().unwrap().join("keywords.md");
  check_all(path.as_path(), check_with_std_basis);
}
