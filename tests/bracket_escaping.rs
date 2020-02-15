use t4rust_derive::Template;

#[derive(Template)]
#[TemplatePath = "./tests/bracket_escaping.tt"]
struct BracketEscaping { }

#[test]
pub fn bracket_escapeing() {
	let f = format!("{}", BracketEscaping{});
	let f = f.trim_end_matches(|c| c == '\r' || c == '\n');
	assert_eq!(f, "This should be safe {}, this { too } {{}} {{}{}}.");
}
