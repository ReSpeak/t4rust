#[macro_use]
extern crate t4rust_derive;

#[derive(Template)]
#[TemplatePath = "./tests/text_only.tt"]
struct TextOnly { }

#[test]
pub fn text_only() {
    format!("{}", TextOnly{});
}