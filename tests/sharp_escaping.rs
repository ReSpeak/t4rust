#[macro_use]
extern crate t4rust_derive;

#[derive(Template)]
#[TemplatePath = "./tests/sharp_escaping.tt"]
struct SharpEscaping { }

#[test]
pub fn sharp_escaping() {
	let f = format!("{}", SharpEscaping{});
	let f = f.trim_end_matches(|c| c == '\r' || c == '\n');
	assert!(f == r####"This should be safe r#""#, this too r###""###."####);
}
