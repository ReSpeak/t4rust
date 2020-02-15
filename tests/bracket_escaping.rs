#[macro_use]
extern crate t4rust_derive;

#[derive(Template)]
#[TemplatePath = "./tests/bracket_escaping.tt"]
struct BracketEscapeing { }

#[test]
pub fn bracket_escapeing() {
    format!("{}", BracketEscapeing{});
}
