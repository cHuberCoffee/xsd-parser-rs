use std::borrow::Cow;

use crate::{
    generator::{validator::gen_validate_impl, Generator},
    parser::types::{Enum, EnumSource, RsEntity},
};

pub trait EnumGenerator {
    fn generate(&self, entity: &Enum, gen: &Generator) -> String {
        let name = self.get_name(entity, gen);
        let default_case = format!(
            "impl Default for {name} {{\n\
            {indent}fn default() -> {name} {{\n\
            {indent}{indent}Self::__Unknown__(\"No valid variants\".into())\n\
            {indent}}}\n\
            }}",
            name = name,
            indent = gen.base().indent()
        );

        let trait_impl = self.impl_fmt_fromstr(entity, gen);

        format!(
            "{comment}{macros}\
            pub enum {name} {{\n\
                {cases}\n\
                {indent}__Unknown__({typename}),\n\
            }}\n\n\
            {default}\n\n\
            {validation}\n\n\
            {traits}\n\n\
            {subtypes}\n\n",
            indent = gen.base().indent(),
            comment = self.format_comment(entity, gen),
            macros = self.macros(entity, gen),
            name = name,
            cases = self.cases(entity, gen),
            typename = self.get_type_name(entity, gen),
            default = default_case,
            traits = trait_impl,
            subtypes = self.subtypes(entity, gen),
            validation = self.validation(entity, gen),
        )
    }

    fn cases(&self, entity: &Enum, gen: &Generator) -> String {
        entity
            .cases
            .iter()
            .map(|case| gen.enum_case_gen().generate(case, gen))
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn subtypes(&self, entity: &Enum, gen: &Generator) -> String {
        gen.base().join_subtypes(entity.subtypes.as_ref(), gen)
    }

    fn get_type_name(&self, entity: &Enum, gen: &Generator) -> String {
        gen.base().format_type_name(entity.type_name.as_str(), gen).into()
    }

    fn get_name(&self, entity: &Enum, gen: &Generator) -> String {
        gen.base().format_type_name(entity.name.as_str(), gen).into()
    }

    fn macros(&self, entity: &Enum, gen: &Generator) -> Cow<'static, str> {
        if entity.source == EnumSource::Union {
            return "#[derive(PartialEq, Clone, Debug, UtilsUnionSerDe)]\n".into();
        }

        let derives = "#[derive(PartialEq, Debug, Clone, YaSerialize, YaDeserialize)]\n";
        let tns = gen.target_ns.borrow();
        match tns.as_ref() {
            Some(tn) => match tn.name() {
                Some(name) => format!(
                    // needs to put it differently when multiples namespaces are defined.
                    // namepsaces = {"tns" = "example.com", "tds = "another.example.com"}
                    "{derives}#[yaserde(prefix = \"{prefix}\", namespaces = {{\"{prefix}\" = \"{uri}\"}})]\n",
                    derives = derives,
                    prefix = name,
                    uri = tn.uri()
                ),
                // deal with it
                None => format!(
                    "{derives}#[yaserde(namespace = \"{uri}\")]\n",
                    derives = derives,
                    uri = tn.uri()
                ),
            },
            None => format!("{derives}#[yaserde()]\n", derives = derives),
        }
        .into()
    }

    fn format_comment(&self, entity: &Enum, gen: &Generator) -> String {
        gen.base().format_comment(entity.comment.as_deref(), 0)
    }

    fn validation(&self, entity: &Enum, gen: &Generator) -> Cow<'static, str> {
        // Empty validation
        Cow::Owned(gen_validate_impl(self.get_name(entity, gen).as_str(), ""))
    }

    fn impl_fmt_fromstr(&self, entity: &Enum, gen: &Generator) -> String {
        // contained_union is an Option<&RsEntity> -> the enum we are operating on is contained in an enum
        let tmp_schema = gen.schema.borrow();
        let contained_union = tmp_schema.types.iter().find(|rse| match rse {
            RsEntity::Enum(x) => {
                let is_contained = x.cases.iter().any(|c| c.name.contains(&entity.name));
                if x.name == entity.name {
                    false
                } else if x.source == EnumSource::Union && is_contained {
                    true
                } else {
                    false
                }
            }
            _ => false,
        });

        if let Some(_) = contained_union {
            let from_str_entities = entity
                .cases
                .iter()
                .map(|case| gen.enum_case_gen().from_string_entities(&entity.name, case))
                .collect::<Vec<String>>()
                .join("\n");
            let fmt_entities = entity
                .cases
                .iter()
                .map(|case| gen.enum_case_gen().fmt_string_entities(&entity.name, case))
                .collect::<Vec<String>>()
                .join("\n");

            let from_str_imp = format!(
                "impl FromStr for {ename} {{\n\
                    type Err = String;\n\n\
                    fn from_str(s: &str) -> Result<Self, Self::Err> {{\n\
                        match s {{\n\
                            {entities}\n\
                            _ => Err(\"Not valid variants\".into()),
                        }}\n\
                    }}\n\
                }}",
                ename = entity.name,
                entities = from_str_entities
            );

            let fmt_imp = format!(
                "impl std::fmt::Display for {ename} {{\n\
                    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {{\n\
                        match self {{\n\
                            {entities}\n\
                            {ename}::__Unknown__(s) => write!(f, \"__Unknown__{{}}\", s),\n\
                        }}\n\
                    }}\n\
                }}",
                ename = entity.name,
                entities = fmt_entities
            );

            format!("{from_str_imp}\n\n{fmt_imp}")
        } else {
            "".to_string()
        }
    }
}

pub struct DefaultEnumGen;
impl EnumGenerator for DefaultEnumGen {}
