use crate::{
    generator::Generator,
    parser::types::{Import, ImportType},
};

pub trait ImportGenerator {
    fn generate(&self, entity: &Import, _gen: &Generator) -> String {
        // include: pub use <name>::*;
        // import : use <name> as <prefix>;

        let crate_name = prepare_create_name(&entity.location);
        match entity.itype {
            ImportType::Include => {
                format!("pub use {}::*;\n", crate_name)
            }
            ImportType::Import => {
                if entity.prefix == None {
                    return "".to_string();
                }

                format!(
                    "use {} as {};\n",
                    crate_name,
                    entity.prefix.as_ref().unwrap().replace("-", "_")
                )
            }
        }
    }
}

fn prepare_create_name(path: &String) -> String {
    let path_parts: Vec<String> = path.split("/").map(|e| e.to_string()).collect();
    let crate_name = if path_parts.last() == Some(&"include".to_string()) {
        path_parts.get(path_parts.len() - 2).unwrap_or(&"".to_string()).clone()
    } else {
        path_parts.last().unwrap_or(&"".to_string()).clone()
    };

    crate_name.replace("-", "_").replace(".xsd", "")
}

pub struct DefaultImportGen;
impl ImportGenerator for DefaultImportGen {}
