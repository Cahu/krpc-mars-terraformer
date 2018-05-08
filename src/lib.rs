extern crate serde_json;
use serde_json as json;

#[macro_use]
extern crate failure;

use std::fs;
use std::io;
use std::ffi;
//use std::fmt;
use std::io::prelude::*;
use std::path::Path;


mod genfailure;
use genfailure::GenFailure;

struct Generator {
    input:  fs::File,
    output: fs::File,
}

pub fn run<T, U>(services_path: T, output_dir: U) -> Result<(), GenFailure>
    where T: AsRef<Path>,
          U: AsRef<Path>
{
    let services_path : &Path = services_path.as_ref();
    let output_dir    : &Path = output_dir.as_ref();

    for entry in fs::read_dir(services_path).map_err(GenFailure::IoFailure)? {
        if let Ok(entry)       = entry {
        if let Ok(file_type)   = entry.file_type() {
        if let Some(file_name) = entry.file_name().to_str() {
            if !file_type.is_file() {
                continue;
            }
            if !file_name.ends_with(".json") {
                continue;
            }
            Generator::new(output_dir, &entry.path())?.run()?;
        }}}
    }

    Ok(())
}


impl Generator {

    fn new(output_dir: &Path, service_file: &Path) -> Result<Self, GenFailure> {
        // Get the base name and append ".rs" to it
        let mut output_file = service_file.file_stem()
            .and_then(ffi::OsStr::to_str).unwrap()
            .to_string();
        output_file.push_str(".rs");

        let input  = fs::File::open(service_file).map_err(GenFailure::IoFailure)?;
        let output = fs::File::create(output_dir.join(output_file)).map_err(GenFailure::IoFailure)?;

        Ok(Generator { input, output })
    }

    fn run(&mut self) -> Result<(), GenFailure> {

        let mut contents = String::new();
        self.input.read_to_string(&mut contents).map_err(GenFailure::IoFailure)?;

        let doc : json::Value = json::from_str(&contents).map_err(GenFailure::JsonFailure)?;

        if let json::Value::Object(map) = doc {
            for (ref service_name, ref service_definition) in map {
                self.generate_code_for_service(service_name, service_definition)?;
            }
            Ok(())
        }
        else {
            Err(GenFailure::ParseError(String::from("Malformed service file")))
        }
    }


    fn generate_code_for_service(&mut self, service_name: &str, service_definition: &json::Value) -> Result<(), GenFailure> {

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

        Ok(())
    }
}
