use t4rust_derive::Template;

#[derive(Template)]
#[TemplatePath = "./examples/recompile.tt"]
struct Example {
    name: String
}

fn main() {
    let result = format!("{}", Example { name: "Splamy".into() });
    println!("{}", result);
}
