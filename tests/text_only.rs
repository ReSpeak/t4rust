use t4rust_derive::Template;

#[derive(Template)]
#[TemplatePath = "./tests/text_only.tt"]
struct TextOnly {}

#[test]
pub fn text_only() {
	let f = format!("{}", TextOnly {});
	let f = f.trim_end_matches(|c| c == '\r' || c == '\n');
	assert_eq!(f, "Hello only Text.");
}
