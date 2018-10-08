# t4rust

[![dependency status](https://deps.rs/repo/github/ReSpeak/t4rust/status.svg)](https://deps.rs/repo/github/ReSpeak/t4rust)

{{readme}}

You can simply write rust code within code blocks.

Code is written within `<#` and `#>` blocks.
If you want to write a `<#` in template text without starting a code block
simply write it twice: `<#<#`. Same goes for the `#>` in code blocks.
You dont need to duplicate the `<#` within code blocks and `#>` not in
template text blocks.

You can use `<#= expr #>` to print out a single expression.

# License
Licensed under either of

 * [Apache License, Version 2.0](LICENSE-APACHE)
 * [MIT license](LICENSE-MIT)

at your option.