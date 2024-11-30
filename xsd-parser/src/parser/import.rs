use roxmltree::Node;

use crate::parser::{
    constants::attribute,
    types::{Import, ImportType, RsEntity},
};

pub fn parse_import(node: &Node, parent: &Node) -> RsEntity {
    // Include does add multiple schemas with the same namespace!
    // Import does add multiple schemas with different namespace!
    RsEntity::Import(Import {
        itype: match node.tag_name().name() {
            "import" => ImportType::Import,
            "include" => ImportType::Include,
            _ => unreachable!("Import Entity must be of type Import or Include"),
        },
        prefix: parent
            .namespaces()
            .filter(|namespace| {
                namespace.uri() == node.attribute(attribute::NAMESPACE).unwrap_or("")
            })
            .find(|namespace| namespace.name().is_some())
            .cloned()
            .and_then(|namespace| namespace.name().map(|namespace| namespace.to_string())),
        name: node.attribute(attribute::NAMESPACE).unwrap_or("").into(),
        location: node.attribute(attribute::SCHEMA_LOCATION).unwrap_or("").into(),
        comment: None,
    })
}
