//! # About
//! t4rust is a minimal templating engine, inspired by the [T4](https://docs.microsoft.com/en-us/visualstudio/modeling/code-generation-and-t4-text-templates) syntax.
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
//!#    assert_eq!(result, "Hello From Template!\nMy Name is: Splamy\nI like to eat Cake.\nNum:1\nNum:2\nNum:3\n");
//! }
//! ```
//!
//! `doc_example1.tt`:
//! ```
//! Hello From Template!
//! My Name is: <# write!(f, "{}", self.name)?; #>
//! I like to eat <#= self.food #>.
//! <# for num in 0..self.num { #>Num:<#= num + 1 #>
//! <# } #>
//! ```
//!
//! Output:
//! ```
//! Hello From Template!
//! My Name is: Splamy
//! I like to eat Cake.
//! Num:1
//! Num:2
//! Num:3
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
use proc_macro::TokenStream;
use syn::*;
use syn::MetaItem::*;
use nom::IResult::*;
use nom::{alphanumeric, space};
use ::TemplatePart::*;

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

    let mut path: Option<String> = None;
    let mut info = TemplateInfo { debug_print: false, clean_whitespace: false };

    for attr in ast.attrs {
        match attr.value {
            NameValue(name, value) => if name == TEMPLATE_PATH_MACRO {
                if let Lit::Str(val_string, _) = value {
                    path = Some(val_string);
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

    // Get template path
    let mut path_absolute = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    path_absolute.push(&path.expect(
        format!("Please specify a #[{}=\"<path>\"] atribute with the template file path.", TEMPLATE_PATH_MACRO).as_str(),
    ));
    let path = &path_absolute.canonicalize().expect("Could not canonicalize path");
    dbg_println!(info, "Looking for template in \"{}\"", path.to_str().unwrap());

    // Read template file
    let read = read_from_file(path).expect("Could not read file");

    // Parse template file
    let mut data = parse_all(&mut info, read.as_bytes()).expect("Parse failed!");

    if info.debug_print {
        debug_to_file(path, &data);
    }

    parse_postprocess(&mut info, &mut data);

    let data = parse_optimize(data);


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
            Directive(_) => {}
        }
    }

    dbg_println!(info, "Generated Code:\n{}", builder);
    let tokens = syn::parse_token_trees(&builder).expect("Parsing template code failed!");

    // Build frame and insert
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;
    let path_str = path.to_str();

    let frame = quote!{
        impl #impl_generics ::std::fmt::Display for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                let _ = include_bytes!(#path_str);
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
                Directive(ref dir) => {
                    write!(file, "Dir:").unwrap();
                    write!(file, "{:?}", dir).unwrap();
                }
            }
            write!(file, "\n").unwrap();
        }
    }
}

/// Transforms template code into an intermediate representation
fn parse_all(info: &mut TemplateInfo, input: &[u8]) -> Result<Vec<TemplatePart>, TemplateError> {
    let mut builder: Vec<TemplatePart> = Vec::new();
    let mut cur = input;

    dbg_println!(info, "Reading template");

    'mloop: while cur.len() > 0 {
        let (crest, content) = parse_text(info, cur)?;
        builder.push(Text(content));
        cur = crest;
        dbg_println!(info, "");

        // Read code block
        if let Done(rest, _) = expression_start(cur) {
            dbg_print!(info, " expression start");
            let (crest, content) = parse_code(info, rest)?;
            builder.push(Expr(content));
            cur = crest;
        } else if let Done(rest, _) = template_directive_start(cur) {
            dbg_print!(info, " directive start");
            let (crest, content) =  parse_code(info, rest)?;
            let dir = parse_directive(&content);
            dbg_println!(info, " Directive: {:?}", dir);
            match dir {
                Done(_, dir) => {
                    apply_directive(info, &dir);
                    builder.push(Directive(dir));
                }
                _ => return Err(TemplateError { index: 0 }),
            }
            cur = crest;
        } else if let Done(rest, _) = code_start(cur) {
            dbg_print!(info, " code start");
            let (crest, content) =  parse_code(info, rest)?;
            builder.push(Code(content));
            cur = crest;
        }

        dbg_println!(info, " Rest: {:?}", String::from_utf8(cur.to_vec()).unwrap());
    }

    dbg_println!(info, "\nTemplate ok!");

    Result::Ok(builder)
}

fn parse_text<'a>(info: &TemplateInfo, input: &'a [u8]) -> Result<(&'a [u8], Vec<u8>), TemplateError> {
    let mut content = Vec::<u8>::new();
    let mut cur = input;

    loop {
        let read = read_text(cur);
        match read {
            Done(rest, done) => {
                content.extend(done);
                if rest.len() == 0 {
                    return Ok((rest, content));
                }
                cur = rest;
                dbg_print!(info, " take text: {:?}", String::from_utf8(done.to_vec()).unwrap());

                if let Done(rest, _) = double_code_start(cur) {
                    dbg_print!(info, " double-escape");
                    content.extend(b"<#");

                    if rest.len() == 0 {
                        return Ok((rest, content));
                    }
                    cur = rest;
                } else if done.len() == 0 {
                    return Ok((rest, content));
                }
            }
            _ => {
                if let Done(rest, done) = till_end(cur) {
                    if rest.len() == 0 {
                        content.extend(done);
                        return Ok((rest, content));
                    }
                }
                match read {
                    Error(err) => dbg_println!(info, "Error at text {:?}", err),
                    Incomplete(n) => dbg_println!(info, "Missing at text {:?}", n),
                    _ => unreachable!(),
                }
                return Err(TemplateError { index: 0 });
            }
        }

        dbg_println!(info, " Rest: {:?}", String::from_utf8(cur.to_vec()));
    }
}

fn parse_code<'a>(info: &TemplateInfo, input: &'a [u8]) -> Result<(&'a [u8], Vec<u8>), TemplateError> {
    let mut content = Vec::<u8>::new();
    let mut cur = input;

    loop {
        match read_code(cur) {
            Done(rest, done) => {
                dbg_print!(info, " take code: {:?}", String::from_utf8(done.to_vec()).unwrap());
                content.extend(done);
                cur = rest;

                if let Done(rest, _) = code_end(cur) {
                    dbg_print!(info, " code end");
                    return Ok((rest, content));
                } else if let Done(rest, _) = double_code_end(cur) {
                    dbg_print!(info, " double-escape");
                    content.extend(b"#>");
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
}

/// Melds multiple identical Parts into one
fn parse_optimize(data: Vec<TemplatePart>) -> Vec<TemplatePart> {
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
            Directive(_) => {}
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

fn parse_postprocess(info: &mut TemplateInfo, data: &mut Vec<TemplatePart>) {
    let mut was_b_clean = None;
    let mut clean_index = 0;

    for i in 0..(data.len() - 2) {
        let tri = data[i..(i+3)].as_mut();
        if let Directive(ref dir) = tri[0] {
            apply_directive(info, dir);
        }

        if !info.clean_whitespace ||
           !tri[0].is_text() || !tri[1].is_code() || !tri[2].is_text() {
            continue;
        }

        let mut res_a = None;
        if clean_index == i && was_b_clean.is_some() {
            res_a = was_b_clean;
        } else if let Text(ref text_a) = tri[0] {
            let rev_txt: Vec<u8> = text_a.iter().rev().cloned().collect();
            if let Done(_,a_len) = is_ws_till_newline(&rev_txt) {
                res_a = Some(a_len);
            } else {
                continue;
            }
        }

        let mut res_b = None;
        if let Text(ref text_b) = tri[2] {
            if let Done(_,b_len) = is_ws_till_newline(&text_b) {
                res_b = Some(b_len);
            } else {
                continue;
            }
        }

        // start trimming

        if let Text(ref mut text_a) = tri[0] {
            let res_a = res_a.unwrap();
            let len = text_a.len();
            text_a.drain((len-(res_a.0))..len);
        }

        if let Text(ref mut text_b) = tri[2] {
            let rev_txt: Vec<u8> = text_b.iter().rev().cloned().collect();
            if let Done(_,b_len) = is_ws_till_newline(&rev_txt) {
                was_b_clean = Some(b_len);
                clean_index = i + 2;
            }

            let res_b = res_b.unwrap();
            text_b.drain(0..(res_b.0 + res_b.1));
        }
    }
}

fn apply_directive(info: &mut TemplateInfo, directive: &TemplateDirective) {
    match directive.name.as_ref() {
        "template" => {
            for &(ref key, ref value) in &directive.params {
                match key.as_ref() {
                    "debug" => info.debug_print = value.parse::<bool>().unwrap(),
                    "cleanws" => info.clean_whitespace = value.parse::<bool>().unwrap(),
                    _ => println!("Unrecognized template parameter \"{}\" in \"{}\"", key, directive.name),
                }
            }
        }
        _ => println!("Unrecognized template dirctive \"{}\"", directive.name),
    }
}

named!(
    code_start,
    do_parse!(first: tag!("<#") >> not!(tag!("<#")) >> (first))
);
named!(expression_start, do_parse!(first: tag!("<#=") >> (first)));
named!(template_directive_start, do_parse!(first: tag!("<#@") >> (first)));
named!(read_text, take_until!("<#"));
named!(double_code_start, tag!("<#<#"));

named!(
    code_end,
    do_parse!(first: tag!("#>") >> not!(tag!("#>")) >> (first))
);
named!(read_code, take_until!("#>"));
named!(double_code_end, tag!("#>#>"));

named!(till_end, take_while!(|_| true));

named!(parse_directive<&[u8], TemplateDirective>, do_parse!(
    opt!(call!(space)) >>
    dir_name : call!(alphanumeric) >>
    dir_param : many0!(call!(parse_directive_param)) >>
    (TemplateDirective { name: String::from_utf8(dir_name.to_vec()).unwrap(), params: dir_param } )
));

named!(not_quote, is_not!("\\\""));

named!(parse_directive_param<&[u8], (String, String) >, do_parse!(
    opt!(call!(space)) >>
    key : call!(alphanumeric) >>
    tag!("=") >>
    tag!("\"") >>
    value : escaped_transform!(call!(not_quote), '\\',
        alt!(
            tag!("\\") => { |_| b"\\".as_ref() }
            | tag!("\"") => { |_| b"\"".as_ref() }
        )) >>
    tag!("\"") >>
    opt!(call!(space)) >>
    ( String::from_utf8(key.to_vec()).unwrap(),
      String::from_utf8(value.to_vec()).unwrap() )
));

named!(is_ws_till_newline<(usize, usize)>,do_parse!(
    lenws: opt!(is_a_s!(" \t")) >>
    lenlb: alt_complete!(tag!("\r\n") | tag!("\n\r") | tag!("\n") | tag!("\r")) >>
    ( if let Some(ws) = lenws { ws.len() } else { 0 } , lenlb.len() ) )
);


#[derive(Debug)]
struct TemplateError {
    index: usize,
}

#[derive(Debug)]
struct TemplateDirective {
    name: String,
    params: Vec<(String,String)>,
}

#[derive(Debug)]
enum TemplatePart {
    Text(Vec<u8>),
    Code(Vec<u8>),
    Expr(Vec<u8>),
    Directive(TemplateDirective),
}

impl TemplatePart {
    fn is_text(&self) -> bool {
        if let &Text(_) = self { true } else { false }
    }

    fn is_code(&self) -> bool {
        if let &Text(_) = self { false } else { true }
    }
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
    clean_whitespace: bool,
}
