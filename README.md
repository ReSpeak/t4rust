# t4rust

[![dependency status](https://deps.rs/repo/github/ReSpeak/t4rust/status.svg)](https://deps.rs/repo/github/ReSpeak/t4rust)

## About
t4rust is a minimal templating engine, inspired by the [T4](https://docs.microsoft.com/en-us/visualstudio/modeling/code-generation-and-t4-text-templates) syntax.

## Example
A simple example how to create a template.

```rust
use t4rust_derive::Template;

// Add this attribute to use a template
#[derive(Template)]
// Specify the path to the template file here
#[TemplatePath = "./examples/doc_example1.tt"]
// Add this attribute if you want to get debug parsing information
// This also enables writing temporary files, you might get better error messages.
//#[TemplateDebug]
struct Example {
    // Add fields to the struct you want to use in the template
    name: String,
    food: String,
    num: i32,
}

fn main() {
    // Generate your template by formating it.
    let result = format!("{}", Example { name: "Splamy".into(), food: "Cake".into(), num: 3 });
    println!("{}", result);
}
```

`doc_example1.tt`:
```
Hello From Template!
My Name is: <# write!(_fmt, "{}", self.name)?; #>
I like to eat <#= self.food #>.
<# for num in 0..self.num { #>Num:<#= num + 1 #>
<# } #>
```

Output:
```
Hello From Template!
My Name is: Splamy
I like to eat Cake.
Num:1
Num:2
Num:3
```

## Syntax

You can simply write rust code within code blocks.

Code is written within `<#` and `#>` blocks.
If you want to write a `<#` in template text without starting a code block
simply write it twice: `<#<#`. Same goes for the `#>` in code blocks.
You dont need to duplicate the `<#` within code blocks and `#>` not in
template text blocks.

You can use `<#= expr #>` to print out a single expression.

Maybe you noticed the magical `_fmt` in the template. This variable gives you
access to the formatter and e.g. enables you to write functions in your
template. `<# write!(_fmt, "{}", self.name)?; #>` is equal to `<#= self.name #>`.

**Warning**: Make sure to never create a variable called `_fmt`! You will get
weird compiler errors.

# License
Licensed under either of

 * [Apache License, Version 2.0](LICENSE-APACHE)
 * [MIT license](LICENSE-MIT)

at your option.
