use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::io::Write;
use std::io::prelude::*;

use tera;
use serde_json as json;

use genfailure::GenFailure;

pub struct Generator {
    service_file: PathBuf,
    output_dir:   PathBuf,
}


impl Generator {

    pub fn new(output_dir: &Path, service_file: &Path) -> Result<Self, GenFailure> {
        let service_file = service_file.to_path_buf();
        let output_dir   = output_dir.to_path_buf();
        Ok(Generator { service_file, output_dir })
    }

    pub fn run(&mut self, templates: &tera::Tera) -> Result<(), GenFailure> {

        let mut contents = String::new();
        let mut input    = fs::File::open(self.service_file.as_path()).map_err(GenFailure::IoFailure)?;
        input.read_to_string(&mut contents).map_err(GenFailure::IoFailure)?;

        let doc : json::Value = json::from_str(&contents).map_err(GenFailure::JsonFailure)?;

        if let json::Value::Object(map) = doc {
            for (ref service_name, ref service_definition) in map {
                self.generate_code_for_service(templates, service_name, service_definition)?;
            }
            Ok(())
        }
        else {
            Err(GenFailure::ParseError(String::from("Malformed service file")))
        }
    }


    fn generate_code_for_service(&mut self, templates: &tera::Tera, service_name: &str, service_definition: &json::Value)
        -> Result<(), GenFailure>
    {
        // Build the path to the output file
        let mut output_file_path = self.output_dir.clone();
        output_file_path.push(service_name.to_lowercase());
        output_file_path.set_extension("rs");

        let mut output = fs::File::create(output_file_path).map_err(GenFailure::IoFailure)?;

        let mut ctx = tera::Context::new();
        ctx.add("service_name",       service_name);
        ctx.add("service_definition", service_definition);
        let rendered = templates.render("service.rs", &ctx).map_err(GenFailure::TemplateFailure)?;

        output.write_all(rendered.as_bytes()).map_err(GenFailure::IoFailure)?;

        /*
        if let json::Value::Object(procedures_map) = &service_definition["procedures"]   {
        if let json::Value::Object(classes_map)    = &service_definition["classes"]      {
        if let json::Value::Object(enums_map)      = &service_definition["enumerations"] {

            for (ref procedure_name, ref procedure_definition) in procedures_map {
                println!("Proc: {}", procedure_name);
            }
            for (ref class_name, ref class_definition) in classes_map {
                println!("Class: {}", class_name);
            }
            for (ref enum_name, ref enum_definition) in enums_map {
                println!("Enum: {}", enum_name);
            }
        }
        else { Err(GenFailure::ParseError(String::from("Could not find the 'enumerations' field")))?; }
        }
        else { Err(GenFailure::ParseError(String::from("Could not find the 'classes' field")))?; }
        }
        else { Err(GenFailure::ParseError(String::from("Could not find the 'procedures' field")))?; }
        */

        Ok(())
    }
}



pub fn type_for() -> tera::GlobalFn {

    fn type_for_aux(val: &tera::Value) -> tera::Result<String> {

        let code = tera::from_value::<String>(val["code"].clone())?.to_lowercase();
        match code.as_ref() {
            "bool"   => Ok(String::from("bool")),
            "double" => Ok(String::from("f64")),
            "float"  => Ok(String::from("f64")),
            "string" => Ok(String::from("String")),
            "sint32" => Ok(String::from("i32")),
            "sint64" => Ok(String::from("i64")),
            "uint32" => Ok(String::from("u32")),
            "uint64" => Ok(String::from("u64")),
            "tuple" => {
                // Recursively call type_for_aux to extrat the types of the tuple's components
                let subtypes : tera::Result<&tera::Value> = val.get("types").ok_or("Missing tuple's 'types' list".into());
                if let tera::Value::Array(subtypes) = subtypes? {
                    let mut sep = "";
                    let mut full_type = String::from("(");
                    for st in subtypes {
                        full_type.push_str(sep);
                        full_type.push_str(&type_for_aux(st)?);
                        sep = ", ";
                    }
                    full_type.push_str(")");

                    Ok(full_type)
                }
                else {
                    Err("Could not extract tuple components".into())
                }
            }
            "list" => {
                let subtypes : tera::Result<&tera::Value> = val.get("types").ok_or("Missing list's 'types' component".into());
                if let tera::Value::Array(subtypes) = subtypes? {
                    // Even though the service files uses an array
                    // there should be only one type defined
                    if subtypes.len() == 1 {
                        let mut full_type = String::from("Vec<");
                        full_type.push_str(&type_for_aux(&subtypes[0])?);
                        full_type.push_str(">");
                        Ok(full_type)
                    }
                    else {
                        Err("Malformed set type".into())
                    }
                }
                else {
                    Err("Could not extract list components".into())
                }
            }
            "set" => {
                let subtypes : tera::Result<&tera::Value> = val.get("types").ok_or("Missing set's 'types' component".into());
                if let tera::Value::Array(subtypes) = subtypes? {
                    // Even though the service files uses an array
                    // there should be only one type defined
                    if subtypes.len() == 1 {
                        let mut full_type = String::from("HashSet<");
                        full_type.push_str(&type_for_aux(&subtypes[0])?);
                        full_type.push_str(">");
                        Ok(full_type)
                    }
                    else {
                        Err("Malformed list type".into())
                    }
                }
                else {
                    Err("Could not extract list components".into())
                }
            }
            "enumeration" | "class" => {
                // TODO: fix this scope issue
                //let service = tera::from_value::<String>(val["service"].clone())?;
                let name    = tera::from_value::<String>(val["name"   ].clone())?;
                /*
                let mut full_type = service;
                full_type.push_str("::");
                full_type.push_str(&name);
                */
                Ok(name)
            }
            "dictionary" => {
                let subtypes : tera::Result<&tera::Value> = val.get("types").ok_or("Missing list's 'types' component".into());
                if let tera::Value::Array(subtypes) = subtypes? {
                    // Even though the service files uses an array there should be only two types
                    // defined : one for the key and one for the value
                    if subtypes.len() == 2 {
                        let mut full_type = String::from("HashMap<");
                        full_type.push_str(&type_for_aux(&subtypes[0])?);
                        full_type.push_str(", ");
                        full_type.push_str(&type_for_aux(&subtypes[1])?);
                        full_type.push_str(">");
                        Ok(full_type)
                    }
                    else {
                        Err("Malformed dictionary type".into())
                    }
                }
                else {
                    Err("Could not extract dictionary components".into())
                }
            }
            t => Err(format!("Unknown type '{}'", t).into()),
        }
    }

    Box::new(move |args| -> tera::Result<tera::Value> {

        let val : tera::Result<&tera::Value> = args.get("type").ok_or("Missing 'type' parameter".into());
        let val = val?;

        let rust_type = type_for_aux(&val)?;

        Ok(tera::to_value(rust_type).unwrap())
    })
}
