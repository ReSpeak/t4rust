use t4rust_derive::Template;

#[derive(Template)]
#[TemplatePath = "./tests/simple_template.tt"]
struct SimpleTemplate {
	text: String
}

#[test]
pub fn simple_template_text() {
	let f = format!("{}", SimpleTemplate{ text: "Inner".into() });
	let f = f.trim_end_matches(|c| c == '\r' || c == '\n');
	assert_eq!(f, "Text Inner Other Text");
}

#[test]
pub fn simple_template_empty() {
	let f = format!("{}", SimpleTemplate{ text: "".into() });
	let f = f.trim_end_matches(|c| c == '\r' || c == '\n');
	assert_eq!(f, "Text  Other Text");
}
