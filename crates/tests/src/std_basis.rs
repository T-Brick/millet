use crate::check::check;

#[test]
fn option() {
  check(
    r#"
val _ = Option.valOf (SOME 3) : int
val _ = Option.getOpt (SOME 3, 123) : int
val _ = Option.getOpt (NONE, false) : bool
val _ = Option.map (fn x => x + 5) (SOME 5) : int option
val _ = Option.map (fn x => x + 5) NONE : int option
val _ = Option.join (SOME (SOME "hey")) : string option
"#,
  );
}

#[test]
fn list() {
  check(
    r#"
val _ = List.length [1, 2] : int
val _ = List.null [] : bool
val _ = List.map (fn x => x = 3) [4, 3, 6] : bool list
"#,
  );
}

#[test]
fn list_pair() {
  check(
    r#"
val _ = ListPair.zip ([1, 4], ["hi", "bye"]) : (int * string) list
"#,
  );
}