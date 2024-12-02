use regex::Regex;

const CARGO_TOML_TEMPLATE: &str = r#"[package]
name = "{pname}"
description = "Autogenerated file"
version = "0.1.0"
edition = "2021"

[dependencies]
{pdependencies}
"#;

pub fn generate_cargo_toml(code: &String, pname: &str) -> String {
    let mut dep: Vec<(String, String)> = vec![
        ("yaserde".to_string(), "{ version = \"0.12\", features = [\"derive\"] }".to_string()),
        ("validate".to_string(), "{ path = \"../../validate\" }".to_string()),
    ];
    if code.contains("UtilsTupleIo")
        || code.contains("UtilsDefaultSerde")
        || code.contains("UtilsUnionSerDe")
    {
        dep.push(("xml-rs".to_string(), "\"0.8\"".to_string()));
    }

    let use_lines = code
        .lines()
        .filter(|line| line.starts_with("use") || line.starts_with("pub use"))
        .filter(|line| !line.contains("yaserde"))
        .filter(|line| !line.contains("std"))
        .filter(|line| !line.contains("validate"));

    let regex = Regex::new(r"(?:use|pub use)\s+([\w_]+)").unwrap();
    let crate_names: Vec<&str> = use_lines
        .filter_map(|line| regex.captures(line))
        .map(|caps| caps.get(1).unwrap().as_str())
        .collect();

    for cn in crate_names {
        let mcn = cn.replace("_", "-");
        if mcn.contains("xsd-") {
            dep.push((
                mcn.to_string(),
                format!(" {{ git = \"https://github.com/lumeohq/xsd-parser-rs\" }}"),
            ));
        } else {
            dep.push((mcn.to_string(), format!(" {{ path = \"./../{}\" }}", mcn)));
        }
    }

    let dep_string = dep
        .iter()
        .map(|(dep_name, dep_version)| format!("{dep_name} = {dep_version}"))
        .collect::<Vec<_>>()
        .join("\n");

    CARGO_TOML_TEMPLATE
        .replace("{pname}", pname)
        .replace("{pdependencies}", &dep_string)
        .to_string()
}
