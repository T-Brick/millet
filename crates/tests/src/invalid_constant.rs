use crate::check::check;

#[test]
fn char() {
  check(
    r#"
val _ = #"あ"
(**     ^^^^^^ invalid character constant *)
"#,
  );
}

#[test]
fn int() {
  check(
    r#"
val _ = 123123123123123123123123132131
(**     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ invalid integer constant: number too large to fit in target type *)
"#,
  );
}

#[test]
fn real() {
  check(
    r#"
val _ = 123.
(**     ^^^^ invalid real constant: invalid float literal *)
"#,
  );
}

#[test]
fn string() {
  check(
    r#"
val _ = "bad \ bad \ bad"
(**     ^^^^^^^ invalid string constant *)
"#,
  );
}