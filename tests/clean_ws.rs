use t4rust_derive::Template;

#[derive(Template)]
#[TemplatePath = "./tests/clean_ws.tt"]
struct CleanWs;

#[derive(Template)]
#[TemplatePath = "./tests/clean_ws2.tt"]
struct CleanWs2;

#[test]
pub fn clean_ws() {
	let f = format!("{}", CleanWs);
	println!("{:?}", f);
	assert_eq!(f, "text\ntext2\ntext3\n\ntext\n\n\ntext2\n\ntext3\n");
}

#[test]
pub fn clean_ws2() {
	let f = format!("{}", CleanWs2);
	println!("{:?}", f);
	assert_eq!(f, "\ntext\n\ntext2\n \ntext3\n\ntext0\n\n\ntext\n \ntext2\n\ntext3\ntext4\n");
}
