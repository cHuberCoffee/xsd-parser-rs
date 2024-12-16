use std::{borrow::Cow, str::FromStr};

use inflector::cases::{pascalcase::to_pascal_case, snakecase::to_snake_case};
use roxmltree::Namespace;

use crate::{generator::function::Function, parser::definitions::Definitions};

pub mod function;

#[derive(Debug, PartialEq)]
pub enum CodeType {
    Client,
    Server,
}

impl FromStr for CodeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "c" => Ok(CodeType::Client),
            "s" => Ok(CodeType::Server),
            _ => Err("<c, s>, c ClientCode, s ServerCode".into()),
        }
    }
}

pub fn generate(definitions: &Definitions) -> String {
    let mut res = vec![];

    for port_type in definitions.port_types().values() {
        for op in port_type.operations() {
            let func = Function::new(op, definitions);
            res.push(generate_function(&func, definitions.target_namespace()));
        }
    }
    res.join("")
}

const IMPL_HEAD: &str = "pub trait MsgHandler {
\tfn handler(&self) -> Result<String, String> {
\t\tErr(format!(\"Method {endp} not implemented\"))
\t}
}\n\n";

const IMPL_BLOCK: &str = "impl MsgHandler for schema::{endp}::{funcname} {
\t//fn handler(&self) -> Result<String, String> {
\t\t// FILL IN IMPLEMENTATION
\t//}
}\n";

pub fn generate_impl_template(definitions: &Definitions, endpoint: &str) -> String {
    let mut impl_blocks: Vec<String> = vec![];
    for port_type in definitions.port_types().values() {
        for operation in port_type.operations() {
            let func = Function::new(operation, definitions);
            impl_blocks.push(
                IMPL_BLOCK
                    .replace("{endp}", endpoint)
                    .replace("{funcname}", &func.name)
                    .to_string(),
            );
        }
    }

    let head = IMPL_HEAD.replace("{endp}", endpoint);
    format!("{head}{blocks}", head = head, blocks = impl_blocks.join("\n"))
}

const DISP_HEAD: &str = "use yaserde::de::from_str;
use super::implementation;

pub fn dispatcher(
\tapp_data: &str,
\tmethod: &str,
) -> Result<Box<dyn implementation::MsgHandler>, String> {
\tmatch method {
{cases}
\t\t_ => Err(format!(\"{} method not found\", method)),
\t}
}";

const DISP_CASE: &str = "\t\t\"{funcname}\" => from_str::<schema::{endp}::{funcname}>(app_data)
\t\t\t.map(|data| Box::new(data) as Box<dyn implementation::MsgHandler>)
\t\t\t.map_err(|e| format!(\"YaDeserialize: {e}\")),";

pub fn generate_dispatcher(definitions: &Definitions, endpoint: &str) -> String {
    let mut cases: Vec<String> = vec![];
    for port_type in definitions.port_types().values() {
        for operation in port_type.operations() {
            let func = Function::new(operation, definitions);
            cases.push(
                DISP_CASE.replace("{endp}", endpoint).replace("{funcname}", func.name).to_string(),
            );
        }
    }

    format!("{head}", head = DISP_HEAD.replace("{cases}", &cases.join("")))
}

const REQUEST_FUNC_BODY: &str = "transport::request(transport, request).await";

fn generate_function(func: &Function<'_>, target_ns: Option<&Namespace>) -> String {
    let ftype = |t| default_format_type(t, target_ns);
    format!(
        r#"
{comment}pub async fn {name}<{generics}>(
    {arguments}
) -> Result<{return_type}, transport::Error> {{
    {body}
}}
"#,
        comment = default_format_comment(func.documentation, 80, 0),
        name = default_format_name(func.name),
        generics = func
            .generic_params
            .iter()
            .map(|p| format!("{}: {}", p.name, ftype(p.typename)))
            .collect::<Vec<String>>()
            .join(", "),
        arguments = func
            .arguments
            .iter()
            .map(|p| format!("{}: &{}", p.name, ftype(p.typename)))
            .collect::<Vec<String>>()
            .join(",\n    "),
        return_type = ftype(func.return_type),
        body = REQUEST_FUNC_BODY
    )
}

fn split_comment_line(s: &str, max_len: usize, indent: usize) -> String {
    let indent_str = " ".repeat(indent);

    let mut splitted = format!("{}//", indent_str);
    let mut current_line_length = indent + 2;
    for word in s.split_whitespace() {
        let len = word.len();
        if current_line_length + len < max_len {
            splitted = format!("{} {}", splitted, word);
            current_line_length += 1 + len;
        } else {
            splitted = format!("{}\n{}// {}", splitted, indent_str, word);
            current_line_length = indent + 3 + len;
        }
    }
    format!("{}\n", splitted)
}

fn default_format_comment(doc: Option<&str>, max_len: usize, indent: usize) -> String {
    doc.unwrap_or("")
        .lines()
        .map(|s| s.trim())
        .filter(|s| s.len() > 1)
        .map(|s| split_comment_line(s, max_len, indent))
        .fold(String::new(), |x, y| (x + &y))
}

fn default_format_type(type_name: &str, target_ns: Option<&Namespace>) -> Cow<'static, str> {
    let (prefix, name) = split_name(type_name);
    let option_tns = target_ns.as_ref().and_then(|ns| ns.name());

    let pascalized_name = filter_type_name(to_pascal_case(name).as_str());

    let qname = |prefix| format!("{}::{}", prefix, pascalized_name);

    let res = match (prefix, option_tns) {
        (Some(ns), Some(tns)) => {
            if ns == tns {
                pascalized_name
            } else {
                qname(ns)
            }
        }
        (Some(ns), None) => qname(ns),
        _ => pascalized_name,
    };

    sanitize(res).into()
}

pub fn default_format_name(name: &str) -> String {
    sanitize(to_snake_case(name.split(':').last().unwrap()))
}

fn split_name(name: &str) -> (Option<&str>, &str) {
    match name.find(':') {
        Some(index) => (Some(&name[0..index]), &name[index + 1..]),
        None => (None, name),
    }
}

fn filter_type_name(name: &str) -> String {
    fn is_valid_symbol(c: char) -> bool {
        (c.is_alphanumeric() || c == '_') && c.is_ascii() && !c.is_whitespace()
    }

    name.chars().filter(|c| is_valid_symbol(*c)).collect()
}

fn sanitize(s: String) -> String {
    if s.is_empty() {
        s
    } else if s.chars().next().unwrap().is_numeric() || RS_KEYWORDS.contains(&s.as_str()) {
        format!("_{}", s)
    } else {
        s
    }
}

const RS_KEYWORDS: &[&str] = &[
    "abstract",
    "alignof",
    "as",
    "async",
    "await",
    "become",
    "box",
    "break",
    "const",
    "continue",
    "crate",
    "do",
    "dyn",
    "else",
    "enum",
    "extern crate",
    "extern",
    "false",
    "final",
    "fn",
    "for",
    "if let",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "macro",
    "match",
    "mod",
    "move",
    "mut",
    "offsetof",
    "override",
    "priv",
    "proc",
    "pub",
    "pure",
    "ref",
    "return",
    "Self",
    "self",
    "sizeof",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "typeof",
    "unsafe",
    "unsized",
    "use",
    "virtual",
    "where",
    "while",
    "yield",
];
