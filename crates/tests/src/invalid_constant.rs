use crate::check::check;

#[test]
fn char_big() {
  check(
    r#"
val _ = #"あ"
(**     ^^^^^^ character literal must have length 1 *)
"#,
  );
}

#[test]
fn char_small() {
  check(
    r#"
val _ = #""
(**     ^^^ character literal must have length 1 *)
"#,
  );
}

#[test]
fn int() {
  check(
    r#"
val _ = 123123123123123123123123132131
(**     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ invalid literal: number too large to fit in target type *)
"#,
  );
}

#[test]
fn real() {
  check(
    r#"
val _ = 123.
(**     ^^^^ incomplete literal *)
"#,
  );
}

#[test]
fn string() {
  check(
    r#"
val _ = "bad \ bad \ bad"
(**     ^^^^^^^ invalid string literal *)
"#,
  );
}
