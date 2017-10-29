#[macro_use]
extern crate t4rust_derive;

#[derive(Template)]
#[TemplatePath = "./examples/recompile.tt"]
#[TemplateDebug]
struct Example {
    name: String
}

fn main() {
    let result = format!("{}", Example { name: "Splamy".into() });
    println!("{}", result);
}
