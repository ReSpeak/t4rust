use t4rust_derive::Template;

#[derive(Template)]
#[TemplatePath = "./tests/sharp_escaping.tt"]
struct SharpEscaping {}

#[test]
pub fn sharp_escaping() {
	let f = format!("{}", SharpEscaping {});
	let f = f.trim_end_matches(|c| c == '\r' || c == '\n');
	assert_eq!(f, r####"This should be safe r#""#, this too r###""###."####);
}
