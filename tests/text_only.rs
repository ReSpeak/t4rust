#[macro_use]
extern crate t4rust_derive;

#[derive(Template)]
#[TemplatePath = "./tests/text_only.tt"]
struct TextOnly { }

#[test]
pub fn text_only() {
	let f = format!("{}", TextOnly{});
	let f = f.trim_end_matches(|c| c == '\r' || c == '\n');
	assert!(f == "Hello only Text.");
}
