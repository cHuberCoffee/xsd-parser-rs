use std::{
    fs,
    io::{prelude::*, Read},
    path::{Path, PathBuf},
    process::Command,
};

use clap::Parser;
use roxmltree::{Document, Node};
use wsdl_parser::{
    generator::{generate, generate_dispatcher, generate_impl_template, CodeType},
    parser::definitions::Definitions,
};
use xsd_parser::{
    generator::{self, builder::GeneratorBuilder},
    parser::schema::parse_schema,
};

#[derive(Parser)]
#[clap(name = env!("CARGO_PKG_NAME"))]
#[clap(version = env!("CARGO_PKG_VERSION"))]
#[clap(about = env!("CARGO_PKG_DESCRIPTION"))]
struct Opt {
    /// Input .wsdl file
    #[clap(long, short)]
    input: PathBuf,

    /// Output path
    #[clap(long, short)]
    output: PathBuf,

    /// Code Type
    #[clap(long, short)]
    codetype: String,

    /// Module Name
    #[clap(long, short)]
    modulename: String,
}

fn main() -> Result<(), String> {
    let opt: Opt = Opt::parse();

    let input_file = opt.input;
    let output_path = opt.output;

    let i_md = fs::metadata(&input_file).map_err(|op| format!("{op}"))?;
    let o_md = fs::metadata(&output_path).map_err(|op| format!("{op}"))?;

    let codetype: CodeType = opt.codetype.parse()?;
    let module = opt.modulename;

    if i_md.is_dir() {
        return Err("Input is not a file".into());
    }

    if !o_md.is_dir() {
        return Err("Ouput is not a path".into());
    }

    process_single_file(&input_file, &output_path, codetype, &module)?;
    Ok(())
}

fn process_single_file(
    input_file: &PathBuf,
    output_path: &PathBuf,
    ct: CodeType,
    modname: &str,
) -> Result<(), String> {
    let text = load_file(input_file).map_err(|e| format!("{e}"))?;
    let doc = Document::parse(text.as_str()).map_err(|e| format!("{e}"))?;
    let definitions = Definitions::new(&doc.root_element());
    let gen = GeneratorBuilder::default().build();

    let schema =
        definitions.types().iter().flat_map(|t| t.schemas()).collect::<Vec<Node<'_, '_>>>();

    let schema_code =
        schema.iter().map(|f| gen.generate_rs_file(&parse_schema(f))).collect::<Vec<String>>();

    let (code, impl_block, dispatcher) = if ct == CodeType::Client {
        let client_code = generate(&definitions);
        let mut client_file = schema_code.clone();
        client_file.push(client_code);

        (client_file.join(""), None, None)
    } else {
        let impl_block = generate_impl_template(&definitions, modname);
        let dispatcher = generate_dispatcher(&definitions, modname);

        (schema_code.join(""), Some(impl_block), Some(dispatcher))
    };

    let cargo = gen.generate_toml_file(&code, modname, generator::toml::FileType::Wsdl);

    let (cargofn, codefn, implfn, dispatcherfn) = if ct == CodeType::Client {
        let mut cargofn = output_path.clone();
        let mut codefn = output_path.clone();
        cargofn.push(format!("{modname}.toml"));
        codefn.push(format!("{modname}.rs"));
        (cargofn, codefn, None, None)
    } else {
        let mut cargofn = output_path.clone();
        let mut codefn = output_path.clone();
        let mut implfn = output_path.clone();
        let mut dispatcherfn = output_path.clone();
        cargofn.push(format!("{modname}.toml"));
        codefn.push(format!("{modname}.rs"));
        implfn.push(format!("{modname}_impl.rs"));
        dispatcherfn.push(format!("{modname}_dispatcher.rs"));
        (cargofn, codefn, Some(implfn), Some(dispatcherfn))
    };

    write_to_file(&cargofn.as_path(), &cargo).map_err(|e| format!("{e}"))?;
    write_to_file(&codefn.as_path(), &code).map_err(|e| format!("{e}"))?;

    format_rust_file(&codefn).map_err(|e| format!("{e}"))?;
    if implfn.is_some() {
        write_to_file(
            &implfn.clone().ok_or(format!("Missing impl block filename"))?.as_path(),
            &impl_block.ok_or(format!("Missing impl block code"))?,
        )
        .map_err(|e| format!("{e}"))?;
        format_rust_file(&implfn.unwrap().as_path()).map_err(|e| format!("{e}"))?;
    }

    if dispatcherfn.is_some() {
        write_to_file(
            &dispatcherfn.clone().ok_or(format!("Missing dispatcher filename"))?.as_path(),
            &dispatcher.ok_or(format!("Missing dispatcher code"))?,
        )
        .map_err(|e| format!("{e}"))?;
        format_rust_file(&dispatcherfn.unwrap().as_path()).map_err(|e| format!("{e}"))?;
    }

    Ok(())
}


fn load_file(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    Ok(text)
}

fn write_to_file(path: &Path, text: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(text.as_bytes())?;

    Ok(())
}

fn format_rust_file(file_path: &Path) -> std::io::Result<()> {
    let output = Command::new("rustfmt")
        .arg("--edition")
        .arg("2021")
        .arg(file_path) // Provide the file directly
        .output()?; // Run rustfmt

    if !output.status.success() {
        eprintln!("rustfmt failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}
