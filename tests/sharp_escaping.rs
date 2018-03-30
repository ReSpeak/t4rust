#[macro_use]
extern crate t4rust_derive;

#[derive(Template)]
#[TemplatePath = "./tests/sharp_escaping.tt"]
struct SharpEscapeing { }

#[test]
pub fn sharp_escapeing() {
    format!("{}", SharpEscapeing{});
}