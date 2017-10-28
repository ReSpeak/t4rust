//! # About
//! t4rust is a minimal templating engine, inspired by the T4 syntax.
//!
//! # Example
//! A simple example how to create a template.
//!
//! ```
//! #[macro_use]
//! extern crate t4rust_derive;
//!
//! // Add this attribute to use a template
//! #[derive(Template)]
//! // Specify the path to the template file here
//! #[TemplatePath = "./examples/doc_example1.tt"]
//! // Add this attribute if you want to get debug parsing information
//! //#[TemplateDebug]
//! struct Example {
//!     // Add fields to the struct you want to use in the template
//!     name: String,
//!     food: String,
//!     num: i32,
//! }
//!
//! fn main() {
//!     // Generate your template by formating it.
//!     let result = format!("{}", Example { name: "Splamy".into(), food: "Cake".into(), num: 3 });
//!     println!("{}", result);
//!     assert_eq!(result, "Hello From Template!\nMy Name is: Splamy\nI like to eat Cake.\nNum:1\nNum:2\nNum:3\n");
//! }
//! ```

#[macro_use]
extern crate nom;
extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

use std::path::Path;
use std::vec::Vec;
use std::path::PathBuf;
use std::fs::File;
use std::io::prelude::*;
use std::result::Result;
use std::option::Option;
use nom::IResult::*;
use proc_macro::TokenStream;
use syn::*;
use syn::MetaItem::*;
use TemplatePart::*;

macro_rules! dbg_println {
    ($inf:ident) => { if $inf.debug_print { println!(); } };
    ($inf:ident, $fmt:expr) => { if $inf.debug_print { println!($fmt); } };
    ($inf:ident, $fmt:expr, $($arg:tt)*) => { if $inf.debug_print { println!($fmt, $($arg)*); } };
}

macro_rules! dbg_print {
    ($inf:ident) => { if $inf.debug_print { print!(); } };
    ($inf:ident, $fmt:expr) => { if $inf.debug_print { print!($fmt); } };
    ($inf:ident, $fmt:expr, $($arg:tt)*) => { if $inf.debug_print { print!($fmt, $($arg)*); } };
}

const TEMPLATE_PATH_MACRO: &str = "TemplatePath";
const TEMPLATE_DEBUG_MACRO: &str = "TemplateDebug";

#[proc_macro_derive(Template, attributes(TemplatePath, TemplateDebug))]
pub fn transform_template(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_derive_input(&s).unwrap();

    let mut path: Option<PathBuf> = None;
    let mut info = TemplateInfo { debug_print: false };

    for attr in ast.attrs {
        match attr.value {
            NameValue(name, value) => if name == TEMPLATE_PATH_MACRO {
                if let Lit::Str(val_string, _) = value {
                    path = Some(PathBuf::from(val_string));
                } else {
                    panic!("[{}] value must be a string.", TEMPLATE_PATH_MACRO);
                }
            },
            Word(name) => if name == TEMPLATE_DEBUG_MACRO {
                info.debug_print = true;
            },
            _ => {}
        }
    }

    let path = &path.expect(
        format!("Please specify a #[{}=\"<path>\"] atribute with the template file path.", TEMPLATE_PATH_MACRO).as_str(),
    );

    // Read template file
    let read = read_from_file(path).expect("Could not read file");

    // Transform template file
    let data = transform(read.as_bytes(), &info).expect("Transform failed!");

    if info.debug_print {
        debug_to_file(path, &data);
    }

    // Build code from template
    let mut builder = String::new();
    for part in data {
        match part {
            Text(x) => {
                builder.push_str(
                    format!("f.write_str(r#\"{}\"#)?;\n", String::from_utf8(x).unwrap()).as_ref(),
                );
            }
            Code(x) => {
                builder.push_str(String::from_utf8(x).unwrap().as_ref());
            }
            Expr(x) => {
                builder.push_str(
                    format!("write!(f, \"{{}}\", {})?;\n", String::from_utf8(x).unwrap()).as_ref(),
                );
            }
        }
    }

    dbg_println!(info, "Generated Code:\n{}", builder);
    let tokens = syn::parse_token_trees(&builder).expect("Parsing template code failed!");

    // Build frame and insert
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;

    let frame = quote!{
        use std::fmt;
        impl #impl_generics fmt::Display for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                #(#tokens)*
                Ok(())
            }
        }
    };

    frame.parse().unwrap()
}

fn read_from_file(path: &Path) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

fn debug_to_file(path: &Path, data: &[TemplatePart]) {
    let mut pathbuf = PathBuf::new();
    pathbuf.push(path);
    pathbuf.set_extension("tt.out");
    let writepath = pathbuf.as_path();
    if let Ok(mut file) = File::create(writepath) {
        for var in data {
            match *var {
                Code(ref x) => {
                    write!(file, "Code:").unwrap();
                    file.write_all(&x).unwrap();
                }
                Text(ref x) => {
                    write!(file, "Text:").unwrap();
                    file.write_all(&x).unwrap();
                }
                Expr(ref x) => {
                    write!(file, "Expr:").unwrap();
                    file.write_all(&x).unwrap();
                }
            }
            write!(file, "\n").unwrap();
        }
    }
}

/// Transforms template code into an intermediate representation
fn transform(input: &[u8], info: &TemplateInfo) -> Result<Vec<TemplatePart>, TemplateError> {
    let mut cur = input;

    let mut builder: Vec<TemplatePart> = Vec::new();

    dbg_println!(info, "Reading template");

    let mut is_text = true;
    let mut is_expr = false;

    'mloop: while cur.len() > 0 {
        if is_text {
            dbg_print!(info, "Templ");
            let read = read_text(cur);
            match read {
                Done(rest, done) => {
                    builder.push(Text(done.to_vec()));
                    dbg_print!(info, " take: {:?}", String::from_utf8(done.to_vec()));
                    cur = rest;

                    if let Done(rest, _) = expression_start(cur) {
                        dbg_print!(info, " xstart");
                        is_text = false;
                        is_expr = true;
                        cur = rest;
                    } else if let Done(rest, _) = code_start(cur) {
                        dbg_print!(info, " cstart");
                        is_text = false;
                        is_expr = false;
                        cur = rest;
                    } else if let Done(rest, _) = double_code_start(cur) {
                        dbg_print!(info, " double");
                        builder.push(Text(b"<#".to_vec()));
                        cur = rest;
                    }
                }
                Error(err) => {
                    if let Done(rest, done) = till_end(cur) {
                        if rest.len() == 0 {
                            builder.push(Text(done.to_vec()));
                            break 'mloop;
                        }
                    }
                    dbg_println!(info, "Error at text {:?}", err);
                    return Err(TemplateError { index: 0 });
                }
                Incomplete(n) => {
                    if let Done(rest, done) = till_end(cur) {
                        if rest.len() == 0 {
                            builder.push(Text(done.to_vec()));
                            break 'mloop;
                        }
                    }
                    dbg_println!(info, "Missing at text {:?}", n);
                    return Err(TemplateError { index: 0 });
                }
            }
        } else {
            dbg_print!(info, "Code");
            match read_code(cur) {
                Done(rest, done) => {
                    if is_expr {
                        builder.push(Expr(done.to_vec()));
                    } else {
                        builder.push(Code(done.to_vec()));
                    }
                    dbg_print!(info, " take: {:?}", String::from_utf8(done.to_vec()));
                    cur = rest;

                    if let Done(rest, _) = code_end(cur) {
                        dbg_print!(info, " cend");
                        is_text = true;
                        cur = rest;
                    } else if let Done(rest, _) = double_code_end(cur) {
                        dbg_print!(info, " double");
                        builder.push(Code(b"#>".to_vec()));
                        cur = rest;
                    }
                }
                Error(err) => {
                    dbg_println!(info, "Error at code {:?}", err);
                    return Err(TemplateError { index: 0 });
                }
                Incomplete(n) => {
                    dbg_println!(info, "Missing at code {:?}", n);
                    return Err(TemplateError { index: 0 });
                }
            }
        }

        dbg_println!(info, " Rest: {:?}", String::from_utf8(cur.to_vec()));
        if cur.len() == 0 {
            break 'mloop;
        }
    }

    dbg_println!(info, "\nTemplate ok!");

    let combined = normalize_transform(builder);
    Result::Ok(combined)
}

/// Melds multiple identical Parts into one
fn normalize_transform(data: Vec<TemplatePart>) -> Vec<TemplatePart> {
    let mut last_type = TemplatePartType::None;
    let mut combined: Vec<TemplatePart> = Vec::new();
    let mut tmp_build: Vec<u8> = Vec::new();
    for item in data {
        match item {
            Code(u) => {
                if u.len() == 0 {
                    continue;
                }
                if last_type != TemplatePartType::Code {
                    if tmp_build.len() > 0 {
                        match last_type {
                            TemplatePartType::None | TemplatePartType::Code => panic!(),
                            TemplatePartType::Text => combined.push(Text(tmp_build)),
                            TemplatePartType::Expr => combined.push(Expr(tmp_build)),
                        }
                    }
                    tmp_build = Vec::new();
                    last_type = TemplatePartType::Code;
                }
                tmp_build.extend(u);
            }
            Text(u) => {
                if u.len() == 0 {
                    continue;
                }
                if last_type != TemplatePartType::Text {
                    if tmp_build.len() > 0 {
                        match last_type {
                            TemplatePartType::None | TemplatePartType::Text => panic!(),
                            TemplatePartType::Code => combined.push(Code(tmp_build)),
                            TemplatePartType::Expr => combined.push(Expr(tmp_build)),
                        }
                    }
                    tmp_build = Vec::new();
                    last_type = TemplatePartType::Text;
                }
                tmp_build.extend(u);
            }
            Expr(u) => {
                if tmp_build.len() > 0 {
                    match last_type {
                        TemplatePartType::None => panic!(),
                        TemplatePartType::Code => combined.push(Code(tmp_build)),
                        TemplatePartType::Text => combined.push(Text(tmp_build)),
                        TemplatePartType::Expr => combined.push(Expr(tmp_build)),
                    }
                }
                tmp_build = Vec::new();
                last_type = TemplatePartType::Expr;
                tmp_build.extend(u);
            }
        }
    }
    if tmp_build.len() > 0 {
        match last_type {
            TemplatePartType::None => {}
            TemplatePartType::Code => combined.push(Code(tmp_build)),
            TemplatePartType::Text => combined.push(Text(tmp_build)),
            TemplatePartType::Expr => combined.push(Expr(tmp_build)),
        }
    }
    combined
}

named!(
    code_start,
    do_parse!(first: tag!("<#") >> not!(tag!("<#")) >> (first))
);
named!(expression_start, do_parse!(first: tag!("<#=") >> (first)));
named!(read_text, take_until!("<#"));
named!(double_code_start, tag!("<#<#"));

named!(
    code_end,
    do_parse!(first: tag!("#>") >> not!(tag!("#>")) >> (first))
);
named!(read_code, take_until!("#>"));
named!(double_code_end, tag!("#>#>"));

named!(till_end, take_while!(|_| true));

#[derive(Debug)]
struct TemplateError {
    index: usize,
}

enum TemplatePart {
    Code(Vec<u8>),
    Text(Vec<u8>),
    Expr(Vec<u8>),
}

#[derive(PartialEq)]
enum TemplatePartType {
    None,
    Code,
    Text,
    Expr,
}

struct TemplateInfo {
    debug_print: bool,
}
