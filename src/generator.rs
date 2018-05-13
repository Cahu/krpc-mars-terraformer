use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::io::prelude::*;

use tera;
use serde_json as json;

use service_generator::ServiceGenerator;
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
            for (service_name, service_definition) in map {
                let mut servicegen = ServiceGenerator::new(&service_name);
                servicegen.run(&self.output_dir, &templates, &service_definition)?;
            }
            Ok(())
        }
        else {
            Err(GenFailure::ParseError(String::from("Malformed service file")))
        }
    }
}
