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
//! ```text
//! Hello From Template!
//! My Name is: <# write!(f, "{}", self.name)?; #>
//! I like to eat <#= self.food #>.
//! <# for num in 0..self.num { #>Num:<#= num + 1 #>
//! <# } #>
//! ```
//!
//! Output:
//! ```text
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
extern crate proc_macro2;
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
use syn::Meta::*;
use nom::{Err, alphanumeric, space};
use nom::types::CompleteStr;
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
    let macro_input = parse_macro_input!(input as DeriveInput);

    let mut path: Option<String> = None;
    let mut info = TemplateInfo { debug_print: false, clean_whitespace: false };

    for attr in macro_input.attrs {
        if let Some(meta) = attr.interpret_meta() {
            match &meta {
                NameValue(MetaNameValue { lit: Lit::Str(lit_str), .. }) =>
                    if meta.name() == TEMPLATE_PATH_MACRO {
                        path = Some(lit_str.value());
                    },
                Word(name) => if name == TEMPLATE_DEBUG_MACRO {
                    info.debug_print = true;
                },
                _ => {}
            }
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
    let mut data = parse_all(&mut info, CompleteStr(&read)).expect("Parse failed!");

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
                builder.push_str(generate_save_str_print(x).as_ref());
            }
            Code(x) => {
                builder.push_str(x.as_ref());
            }
            Expr(x) => {
                builder.push_str(
                    format!("write!(f, \"{{}}\", {})?;\n", x).as_ref()
                );
            }
            Directive(_) => {}
        }
    }

    dbg_println!(info, "Generated Code:\n{}", builder);
    
    //let tokens = syn::parse_str::<UnitStruct>(&builder).expect("Parsing template code failed!");
    let tokens: proc_macro2::TokenStream = builder.parse().expect("Parsing template code failed!");

    // Build frame and insert
    let (impl_generics, ty_generics, where_clause) = macro_input.generics.split_for_impl();
    let name = &macro_input.ident;
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

    proc_macro::TokenStream::from(frame)
}

fn generate_save_str_print(print_str: String) -> String {
    let mut max_sharp_count = 0;
    let mut cur_sharp_count = 0;

    for c in print_str.chars() {
        if c == '#' {
            cur_sharp_count += 1;
            max_sharp_count = std::cmp::max(max_sharp_count, cur_sharp_count);
        }
    }

    let sharps = "#".repeat(max_sharp_count + 1);
    format!("f.write_str(r{1}\"{0}\"{1})?;\n", print_str, sharps)
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
                    file.write_all(x.as_bytes()).unwrap();
                }
                Text(ref x) => {
                    write!(file, "Text:").unwrap();
                    file.write_all(x.as_bytes()).unwrap();
                }
                Expr(ref x) => {
                    write!(file, "Expr:").unwrap();
                    file.write_all(x.as_bytes()).unwrap();
                }
                Directive(ref dir) => {
                    write!(file, "Dir:{:?}", dir).unwrap();
                }
            }
            write!(file, "\n").unwrap();
        }
    }
}

/// Transforms template code into an intermediate representation
fn parse_all(info: &mut TemplateInfo, input: CompleteStr) -> Result<Vec<TemplatePart>, TemplateError> {
    let mut builder: Vec<TemplatePart> = Vec::new();
    let mut cur = input;

    dbg_println!(info, "Reading template");

    while cur.len() > 0 {
        let (crest, content) = parse_text(info, cur)?;
        builder.push(Text(content));
        cur = crest;
        dbg_println!(info, "");

        // Read code block
        if let Ok((rest, _)) = expression_start(cur) {
            dbg_print!(info, " expression start");
            let (crest, content) = parse_code(info, rest)?;
            builder.push(Expr(content));
            cur = crest;
        } else if let Ok((rest, _)) = template_directive_start(cur) {
            dbg_print!(info, " directive start");
            let (crest, content) =  parse_code(info, rest)?;
            let dir = parse_directive(CompleteStr(&content));
            dbg_println!(info, " Directive: {:?}", dir);
            match dir {
                Ok((_, dir)) => {
                    apply_directive(info, &dir);
                    builder.push(Directive(dir));
                }
                _ => return Err(TemplateError { index: 0 }),
            }
            cur = crest;
        } else if let Ok((rest, _)) = code_start(cur) {
            dbg_print!(info, " code start");
            let (crest, content) =  parse_code(info, rest)?;
            builder.push(Code(content));
            cur = crest;
        }

        dbg_println!(info, " Rest: {:?}", &cur);
    }

    dbg_println!(info, "\nTemplate ok!");

    Result::Ok(builder)
}

fn parse_text<'a>(info: &TemplateInfo, input: CompleteStr<'a>) -> Result<(CompleteStr<'a>, String), TemplateError> {
    let mut content = String::new();
    let mut cur = input;

    loop {
        let read = read_text(cur);
        match read {
            Ok((rest, done)) => {
                content.push_str(&done);
                if rest.len() == 0 {
                    return Ok((rest, content));
                }
                cur = rest;
                dbg_print!(info, " take text: {:?}", &done);

                if let Ok((rest, _)) = double_code_start(cur) {
                    dbg_print!(info, " double-escape");
                    content.push_str("<#");

                    if rest.len() == 0 {
                        return Ok((rest, content));
                    }
                    cur = rest;
                } else if done.len() == 0 {
                    return Ok((rest, content));
                }
            }
            _ => {
                if let Ok((rest, done)) = till_end(cur) {
                    if rest.len() == 0 {
                        content.push_str(&done);
                        return Ok((rest, content));
                    }
                }
                match read {
                    Err(Err::Failure(context)) | Err(Err::Error(context)) => dbg_println!(info, "Error at text {:?}", context),
                    Err(Err::Incomplete(sizey)) => dbg_println!(info, "Missing at text {:?}", sizey),
                    _ => unreachable!(),
                }
                return Err(TemplateError { index: 1 });
            }
        }

        dbg_println!(info, " Rest: {:?}", &cur);
    }
}

fn parse_code<'a>(info: &TemplateInfo, input: CompleteStr<'a>) -> Result<(CompleteStr<'a>, String), TemplateError> {
    let mut content = String::new();
    let mut cur = input;

    loop {
        match read_code(cur) {
            Ok((rest, done)) => {
                dbg_print!(info, " take code: {:?}", &done);
                content.push_str(&done);
                cur = rest;

                if let Ok((rest, _)) = code_end(cur) {
                    dbg_print!(info, " code end");
                    return Ok((rest, content));
                } else if let Ok((rest, _)) = double_code_end(cur) {
                    dbg_print!(info, " double-escape");
                    content.push_str("#>");
                    cur = rest;
                }
            }
            Err(Err::Failure(context)) | Err(Err::Error(context)) => {
                dbg_println!(info, "Error at code {:?}", context);
                return Err(TemplateError { index: 2 });
            }
            Err(Err::Incomplete(sizey)) => {
                dbg_println!(info, "Missing at code {:?}", sizey);
                return Err(TemplateError { index: 3 });
            }
        }
    }
}

/// Merges multiple identical Parts into one
fn parse_optimize(data: Vec<TemplatePart>) -> Vec<TemplatePart> {
    let mut last_type = TemplatePartType::None;
    let mut combined = Vec::<TemplatePart>::new();
    let mut tmp_build = String::new();
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
                    tmp_build = String::new();
                    last_type = TemplatePartType::Code;
                }
                tmp_build.push_str(&u);
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
                    tmp_build = String::new();
                    last_type = TemplatePartType::Text;
                }
                tmp_build.push_str(&u);
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
                tmp_build = String::new();
                last_type = TemplatePartType::Expr;
                tmp_build.push_str(&u);
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

/// Applies template directives like 'cleanws' and modifies the input
/// accordingly.
fn parse_postprocess(info: &mut TemplateInfo, data: &mut Vec<TemplatePart>) {
    let mut was_b_clean = None;
    let mut clean_index = 0;

    // if there are less than 3 blocks available we can't do any transformations
    if data.len() < 3 { return; }

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
            let rev_txt: String = text_a.chars().rev().collect();
            if let Ok((_,a_len)) = is_ws_till_newline(CompleteStr(&rev_txt)) {
                res_a = Some(a_len);
            } else {
                continue;
            }
        }

        let mut res_b = None;
        if let Text(ref text_b) = tri[2] {
            if let Ok((_,b_len)) = is_ws_till_newline(CompleteStr(&text_b)) {
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
            let rev_txt: String = text_b.chars().rev().collect();
            if let Ok((_,b_len)) = is_ws_till_newline(CompleteStr(&rev_txt)) {
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
    code_start<CompleteStr, CompleteStr>,
    do_parse!(first: tag!("<#") >> not!(tag!("<#")) >> (first))
);
named!(expression_start<CompleteStr, CompleteStr>, do_parse!(first: tag!("<#=") >> (first)));
named!(template_directive_start<CompleteStr, CompleteStr>, do_parse!(first: tag!("<#@") >> (first)));
named!(read_text<CompleteStr, CompleteStr>, take_until!("<#"));
named!(double_code_start<CompleteStr, CompleteStr>, tag!("<#<#"));

named!(
    code_end<CompleteStr, CompleteStr>,
    do_parse!(first: tag!("#>") >> not!(tag!("#>")) >> (first))
);
named!(read_code<CompleteStr, CompleteStr>, take_until!("#>"));
named!(double_code_end<CompleteStr, CompleteStr>, tag!("#>#>"));

named!(till_end<CompleteStr, CompleteStr>, take_while!(|_| true));

named!(parse_directive<CompleteStr, TemplateDirective>, do_parse!(
    opt!(call!(space)) >>
    dir_name : call!(alphanumeric) >>
    dir_param : many0!(call!(parse_directive_param)) >>
    (TemplateDirective { name: dir_name.to_string(), params: dir_param } )
));

named!(not_quote<CompleteStr, CompleteStr>, is_not!("\\\""));

named!(parse_directive_param<CompleteStr, (String, String)>, do_parse!(
    opt!(call!(space)) >>
    key : call!(alphanumeric) >>
    tag!("=") >>
    tag!("\"") >>
    value : escaped_transform!(call!(not_quote), '\\',
        alt!(
              tag!("\\") => { |_| "\\" }
            | tag!("\"") => { |_| "\"" }
        )) >>
    tag!("\"") >>
    opt!(call!(space)) >>
    (key.to_string(), value.to_string())
));

named!(is_ws_till_newline<CompleteStr, (usize, usize)>,do_parse!(
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
    Text(String),
    Code(String),
    Expr(String),
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
