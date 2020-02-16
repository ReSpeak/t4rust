use t4rust_derive::Template;

#[derive(Template)]
#[TemplatePath = "./tests/escape_function.tt"]
struct EscapeFunction {
	comment: String,
}

fn filter_bad_word(s: &str) -> String { s.replace("peck", "****") }

#[test]
fn escape_function() {
	let f = format!(
		"{}",
		EscapeFunction {
			comment: "This comment does not contain the word peck, oh wait"
				.into()
		}
	);
	let f = f.trim_end_matches(|c| c == '\r' || c == '\n');

	assert_eq!(
		f,
		"\nThis comment section is completely bad word free:
 - This comment does not contain the word ****, oh wait
This comment is unfiltered:
 - This comment does not contain the word peck, oh wait"
	);
}
