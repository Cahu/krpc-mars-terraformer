use std::collections::BTreeMap;
use std::collections::HashSet;

use std::path::Path;

use heck::ToSnakeCase;

/// Errors than can occur when loading Json service files.
#[derive(Debug, thiserror::Error)]
pub enum LoadServiceFileError {
    #[error(transparent)]
    IOErr(#[from] std::io::Error),
    #[error(transparent)]
    JsonErr(#[from] serde_json::Error),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ServiceFile {
    #[serde(flatten)]
    pub services: BTreeMap<String, Service>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Service {
    pub id: u32,
    pub documentation: String,
    pub procedures: BTreeMap<String, Procedure>,
    pub classes: BTreeMap<String, Class>,
    pub enumerations: BTreeMap<String, Enum>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Procedure {
    pub id: u32,
    pub documentation: String,
    pub parameters: Vec<ProcParameter>,
    pub return_type: Option<Type>,
    #[serde(default)]
    pub return_is_nullable: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Class {
    pub documentation: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ProcParameter {
    pub name: String,
    pub r#type: Type,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "code")]
#[serde(rename_all = "UPPERCASE")]
pub enum Type {
    Bool,
    SInt32,
    UInt32,
    Double,
    Float,
    String,
    List { types: Vec<Type> },
    Tuple { types: Vec<Type> },
    Class { service: String, name: String },
    Enumeration { service: String, name: String },
    Dictionary { types: Vec<Type> },
    Set { types: Vec<Type> },
}

impl Type {
    pub fn to_rust_type(&self) -> String {
        match self {
            Type::Bool => "bool".to_string(),
            Type::SInt32 => "i32".to_string(),
            Type::UInt32 => "u32".to_string(),
            Type::Double => "f64".to_string(),
            Type::Float => "f32".to_string(),
            Type::String => "String".to_string(),
            Type::List { types } => {
                let member = types.first().expect("Malformed list type").to_rust_type();
                format!("Vec<{}>", member)
            }
            Type::Tuple { types } => {
                let members = types.iter().map(Type::to_rust_type).collect::<Vec<_>>();
                let members = members.join(", ");
                format!("({members})")
            }
            Type::Class { service, name } => {
                format!("super::{}::{}", service.to_snake_case(), name)
            }
            Type::Enumeration { service, name } => {
                format!("super::{}::{}", service.to_snake_case(), name)
            }
            Type::Dictionary { types } => {
                let mut types = types.into_iter();
                let key_type = types
                    .next()
                    .expect("Malformed dictionary type")
                    .to_rust_type();
                let val_type = types
                    .next()
                    .expect("Malformed dictionary type")
                    .to_rust_type();
                format!("std::collections::HashMap<{key_type}, {val_type}>")
            }
            Type::Set { types } => {
                let member = types.first().expect("Malformed set type").to_rust_type();
                format!("std::collections::HashSet<{member}>")
            }
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Enum {
    pub documentation: String,
    pub values: Vec<EnumValue>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EnumValue {
    pub name: String,
    pub value: u32,
}

impl ServiceFile {
    /// Parse a json service file.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, LoadServiceFileError> {
        let file = std::fs::File::open(path)?;
        let file = serde_json::from_reader(file)?;
        Ok(file)
    }

    /// Generate the file's set of dependencies to other services' classes or enums.
    pub fn get_deps(&self) -> HashSet<String> {
        let mut deps = HashSet::new();
        for (service_name, service_def) in &self.services {
            for proc in service_def.procedures.values() {
                // Types are a recursive structure. We use a stack to keep track of the exploration
                let mut types_stack = Vec::new();
                for param in &proc.parameters {
                    types_stack.push(&param.r#type);
                }
                if let Some(return_type) = &proc.return_type {
                    types_stack.push(&return_type);
                }
                while let Some(ty) = types_stack.pop() {
                    match ty {
                        Type::List { types } => types_stack.extend(types),
                        Type::Tuple { types } => types_stack.extend(types),
                        Type::Dictionary { types } => types_stack.extend(types),
                        Type::Set { types } => types_stack.extend(types),
                        Type::Class { service, name } => {
                            if service != service_name {
                                deps.insert(format!(
                                    "super::{}::{}",
                                    service.to_snake_case(),
                                    name
                                ));
                            }
                        }
                        Type::Enumeration { service, name } => {
                            if service != service_name {
                                deps.insert(format!(
                                    "super::{}::{}",
                                    service.to_snake_case(),
                                    name
                                ));
                            }
                        }
                        _ => { /* nothing to do for primitive types */ }
                    }
                }
            }
        }
        deps
    }
}
