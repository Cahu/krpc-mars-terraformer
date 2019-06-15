use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::collections::HashSet;
use std::collections::HashMap;

use tera;
use serde_json as json;

use heck::SnakeCase;

use error::Error;
use error::Result;

pub struct ServiceGenerator {
    service_name: String,
    procedures:   Vec<tera::Context>,
    methods:      HashMap<String, Vec<tera::Context>>,
    includes:     HashSet<String>, // dependencies with other services
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum TypeKind {
    Primitive,
    Tuple,
    Enum,
    List,
    Dict,
    Set,
    Class
}

struct TypeDef {
    name: String,
    kind: TypeKind,
}


impl ServiceGenerator
{
    pub fn new(service_name: &str) -> Self {
        ServiceGenerator {
            service_name: service_name.to_string(),
            procedures:   Vec::new(),
            methods:      HashMap::new(),
            includes:     HashSet::new(),
        }
    }

    pub fn run(&mut self, output_dir: &PathBuf, templates: &tera::Tera, service_definition: &json::Value) -> Result<()>
    {
        self.includes.clear();

        let procedures_map = service_definition["procedures"].as_object()
            .ok_or(Error::Parse("Could not find the 'procedures' field".to_string()))?;

        let classes_map = service_definition["classes"].as_object()
            .ok_or(Error::Parse("Could not find the 'classes' field".to_string()))?;

        let enumerations_map = service_definition["enumerations"].as_object()
            .ok_or(Error::Parse("Could not find the 'enumerations' field".to_string()))?;

        // Procedures need a bit of pre-processing
        for (ref proc_name, ref proc_def) in procedures_map {
            self.parse_procedure(proc_name, proc_def)?;
        }

        let mut ctx = tera::Context::new();
        ctx.add("service_name", &self.service_name);
        ctx.add("classes",      &classes_map);
        ctx.add("enumerations", &enumerations_map);
        ctx.add("procedures",   &self.procedures);
        ctx.add("methods",      &self.methods);
        ctx.add("includes",     &self.includes);

        let rendered = templates.render("service.rs", &ctx).map_err(Error::Template)?;

        // Build the path to the output file
        let mut output_file_path = output_dir.to_path_buf();
        output_file_path.push(self.service_name.to_snake_case());
        output_file_path.set_extension("rs");

        let mut output = fs::File::create(output_file_path).map_err(Error::Io)?;
        output.write_all(rendered.as_bytes()).map_err(Error::Io)?;

        Ok(())
    }
     

    fn parse_procedure(&mut self, proc_name: &str, proc_def: &json::Value) -> Result<()> {

        let mut parameters = Vec::new();

        let mut doc = String::new();
        if let json::Value::String(doc_str) = &proc_def["documentation"] {
            doc = doc_str.trim().replace("\n", " ");
        }

        let mut ctx = tera::Context::new();
        ctx.add("rpc_name", &proc_name);
        ctx.add("name",     &proc_name.to_snake_case());
        ctx.add("doc",      &doc);

        // A series of checks to identify methods.
        let mut object_name = None;
        if let json::Value::Array(params) = &proc_def["parameters"] {
            // The first param must be called 'this'
            if params.len() > 0 && params[0]["name"] == "this" {
                // Get the type of the 'this' param
                if let json::Value::String(param_type) = &params[0]["type"]["name"] {
                    // Make sure the method name stats with the same name as the above type
                    if proc_name.starts_with(param_type) {
                        // Cleanup the name to remove the prefix
                        let parts : Vec<_> = proc_name.splitn(2, '_').collect();
                        if parts.len() == 2 {
                            object_name = Some(parts[0].to_string());
                            ctx.add("name", &parts[1].to_snake_case());
                        }
                    }
                }
            }

            if object_name.is_none() {
                for param in params {
                    parameters.push(self.parse_param(param)?);
                }
            }
            else {
                for param in &params[1..] {
                    parameters.push(self.parse_param(param)?);
                }
            }

            ctx.add("params", &parameters);
        }

        // Return type
        let return_type = &proc_def["return_type"];
        if !return_type.is_null() {
            let mut return_ctx = tera::Context::new();

            if return_type.is_object() {
                let return_type = self.parse_type(return_type)?;
                return_ctx.add("type", &return_type.name);
                if return_type.kind == TypeKind::Class {
                    return_ctx.add("is_class", &true);
                } else {
                    return_ctx.add("is_class", &false);
                }

            }

            if let json::Value::Bool(truth) = &proc_def["return_type_is_nullable"] {
                return_ctx.add("is_nullable", &truth);
            }

            ctx.add("return", &return_ctx);
        }

        if let Some(impl_name) = object_name {
            let v = self.methods.entry(impl_name).or_insert(Vec::new());
            v.push(ctx);
        }
        else {
            self.procedures.push(ctx);
        }

        Ok(())
    }


    fn parse_param(&mut self, param: &json::Value) -> Result<tera::Context> {
        if let json::Value::String(param_name) = &param["name"] {
            let mut param_ctx = tera::Context::new();
            param_ctx.add("name", &param_name.to_snake_case());

            let param_type = self.parse_type(&param["type"])?;
            match param_type.kind {
                TypeKind::Primitive | TypeKind::Tuple | TypeKind::Enum => {
                    param_ctx.add("type", &param_type.name);
                }
                _ => {
                    param_ctx.add("type", &format!("&{}", param_type.name));
                }
            }
            Ok(param_ctx)
        }
        else {
            Err(Error::Parse(String::from("Could not extract parameter's name")))
        }
    }


    fn parse_type(&mut self, param_type: &json::Value) -> Result<TypeDef> {

        let type_code = param_type["code"].as_str()
            .ok_or(Error::Parse(String::from("type's 'code' not found")))?
            .to_lowercase() ;

        match type_code.as_str() {
            "bool"   => Ok( TypeDef { name: "bool".to_string(),   kind: TypeKind::Primitive }),
            "string" => Ok( TypeDef { name: "String".to_string(), kind: TypeKind::Primitive }),
            "float"  => Ok( TypeDef { name: "f32".to_string(),    kind: TypeKind::Primitive }),
            "double" => Ok( TypeDef { name: "f64".to_string(),    kind: TypeKind::Primitive }),
            "sint32" => Ok( TypeDef { name: "i32".to_string(),    kind: TypeKind::Primitive }),
            "sint64" => Ok( TypeDef { name: "i64".to_string(),    kind: TypeKind::Primitive }),
            "uint32" => Ok( TypeDef { name: "u32".to_string(),    kind: TypeKind::Primitive }),
            "uint64" => Ok( TypeDef { name: "u64".to_string(),    kind: TypeKind::Primitive }),
            "bytes" =>  Ok( TypeDef { name: "&[u8]".to_string(),  kind: TypeKind::Primitive }),
            "tuple" => {
                // Recursively call parse_type to extrat the types of the tuple's components
                if let json::Value::Array(subtypes) = &param_type["types"] {
                    let mut sep = "";
                    let mut name = String::from("(");
                    for st in subtypes {
                        let subtype_def = self.parse_type(st)?;
                        name.push_str(sep);
                        name.push_str(&subtype_def.name);
                        sep = ", ";
                    }
                    name.push_str(")");

                    Ok(TypeDef { name, kind: TypeKind::Tuple })
                }
                else {
                    Err(Error::Parse(String::from("Could not extract 'tuple' components")))
                }
            }
            "list" => {
                if let json::Value::Array(subtypes) = &param_type["types"] {
                    // Even though the service files uses an array
                    // there should be only one type defined
                    if subtypes.len() == 1 {
                        let subtype_def = self.parse_type(&subtypes[0])?;
                        let name = format!("Vec<{}>", &subtype_def.name);
                        Ok(TypeDef { name, kind: TypeKind::List })
                    }
                    else {
                        Err(Error::Parse(String::from("Malformed set type")))
                    }
                }
                else {
                    Err(Error::Parse(String::from("Could not extract 'list' components")))
                }
            }
            "set" => {
                if let json::Value::Array(subtypes) = &param_type["types"] {
                    // Even though the service files uses an array
                    // there should be only one type defined
                    if subtypes.len() == 1 {
                        let subtype_def = self.parse_type(&subtypes[0])?;
                        let name = format!("HashSet<{}>", &subtype_def.name);
                        self.includes.insert("std::collections::HashSet".to_string());
                        Ok(TypeDef { name, kind: TypeKind::Set })
                    }
                    else {
                        Err(Error::Parse(String::from("Malformed 'list' type")))
                    }
                }
                else {
                    Err(Error::Parse(String::from("Could not extract 'list' components")))
                }
            }
            "enumeration" => {
                if let json::Value::String(name) = &param_type["name"] {
                    let mut full_name = name.to_string();
                    if let json::Value::String(service) = &param_type["service"] {
                        if *service != self.service_name {
                            let scope = service.to_snake_case();
                            full_name = format!("{}::{}", scope, full_name);
                            self.includes.insert(scope);
                        }
                    }
                    Ok(TypeDef { name: full_name, kind: TypeKind::Enum })
                }
                else {
                    Err(Error::Parse(String::from("Could not extract 'enumeration' components")))
                }
            }
            "class" => {
                if let json::Value::String(name) = &param_type["name"] {
                    let mut full_name = name.to_string();
                    if let json::Value::String(service) = &param_type["service"] {
                        if *service != self.service_name {
                            let scope = service.to_snake_case();
                            full_name = format!("{}::{}", scope, full_name);
                            self.includes.insert(scope);
                        }
                    }
                    Ok(TypeDef { name: full_name, kind: TypeKind::Class })
                }
                else {
                    Err(Error::Parse(String::from("Could not extract 'class' components")))
                }
            }
            "dictionary" => {
                if let json::Value::Array(subtypes) = &param_type["types"] {
                    // Even though the service files uses an array there should be only two types
                    // defined : one for the key and one for the value
                    if subtypes.len() == 2 {
                        let subtype1 = self.parse_type(&subtypes[0])?;
                        let subtype2 = self.parse_type(&subtypes[1])?;
                        let name = format!("HashMap<{}, {}>", subtype1.name, subtype2.name);
                        self.includes.insert("std::collections::HashMap".to_string());
                        Ok(TypeDef { name, kind: TypeKind::Dict })
                    }
                    else {
                        Err(Error::Parse(String::from("Malformed 'dictionary' type")))
                    }
                }
                else {
                    Err(Error::Parse(String::from("Could not extract 'dictionary' components")))
                }
            }
            "event" => { Ok(TypeDef { name: "::krpc_mars::krpc::Event".to_string(), kind: TypeKind::Primitive }) }
            "procedure_call" => { Ok(TypeDef { name: "::krpc_mars::krpc::ProcedureCall".to_string(), kind: TypeKind::Primitive }) }
            "stream" => { Ok(TypeDef { name: "::krpc_mars::krpc::Stream".to_string(), kind: TypeKind::Primitive }) }
            "services" => { Ok(TypeDef { name: "::krpc_mars::krpc::Services".to_string(), kind: TypeKind::Primitive }) }
            "status" => { Ok(TypeDef { name: "::krpc_mars::krpc::Status".to_string(), kind: TypeKind::Primitive }) }
            t => Err(Error::Parse(format!("Unknown type '{}'", t))),
        }
    }
}
