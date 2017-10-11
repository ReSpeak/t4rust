# t4rust

## About
t4rust is a minimal templating engine, inspired by the [T4](https://docs.microsoft.com/en-us/visualstudio/modeling/code-generation-and-t4-text-templates) syntax.

## Getting started
A simple example how to create a template.

`main.rs`:
```rust
#[macro_use]
extern crate t4rust_derive;

// Add this attribute to use a template
#[derive(Templatable)]
// Specify the path to the template file here
#[TemplatablePath = "./mytemplate.tt"]
struct Example {
    // Add fields to the struct you want to use in the template
    name: String,
    food: String,
}

fn main() {
    println!("{}", Example { name: "Splamy".into(), food: "Cake".into() });
}
```

`mytemplate.tt`:
```rust
Hello From Template!
My Name is: <# write!(f, "{}", self.name)?; #>
I like to eat <#= self.food #>
```

Output:
```
Hello From Template!
My Name is: Splamy
I like to eat Cake
```

You can simply write rust code withing code blocks.

Code is written within `<#` and `#>` blocks.
If you want to write a `<#` in template text without starting a code block
simply write it twice: `<#<#`. Same goes for the `#>` in code blocks.
You dont need to duplicate the `<#` within code blocks and `#>` not in
template text blocks.