use t4rust_derive::Template;

#[derive(Template)]
#[TemplatePath = "./tests/clean_ws.tt"]
struct CleanWs {}

#[test]
pub fn clean_ws() {
	let f = format!("{}", CleanWs {});
	println!("{:?}", f);
	assert_eq!(f, "text\ntext2\ntext3\n\ntext\n\n\ntext2\n\ntext3\n");
}
