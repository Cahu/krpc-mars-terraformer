use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::io::prelude::*;

use tera;
use serde_json as json;

use error::Error;
use error::Result;
use service_generator::ServiceGenerator;

pub struct Generator {
    service_file: PathBuf,
    output_dir:   PathBuf,
}

impl Generator {

    pub fn new(output_dir: &Path, service_file: &Path) -> Result<Self> {
        let service_file = service_file.to_path_buf();
        let output_dir   = output_dir.to_path_buf();
        Ok(Generator { service_file, output_dir })
    }

    pub fn run(&mut self, templates: &tera::Tera) -> Result<()> {

        let mut contents = String::new();
        let mut input    = fs::File::open(self.service_file.as_path()).map_err(Error::Io)?;
        input.read_to_string(&mut contents).map_err(Error::Io)?;

        let doc : json::Value = json::from_str(&contents).map_err(Error::Json)?;

        if let json::Value::Object(map) = doc {
            for (service_name, service_definition) in map {
                let mut servicegen = ServiceGenerator::new(&service_name);
                servicegen.run(&self.output_dir, &templates, &service_definition)?;
            }
            Ok(())
        }
        else {
            Err(Error::Parse(String::from("Malformed service file")))
        }
    }
}
