use std::fs;
use std::path::Path;

extern crate tera;
extern crate serde_json;

mod generator;
use generator::Generator;

mod genfailure;
use genfailure::GenFailure;


pub fn run<T, U>(services_path: T, output_dir: U) -> Result<(), GenFailure>
    where T: AsRef<Path>,
          U: AsRef<Path>
{
    // Compile templates
    let mut templates = tera::Tera::default();
    templates.add_raw_template("service.rs", include_str!("../templates/service.rs.tera")).unwrap();

    // Add some custom functions
    templates.register_global_function("type_for", generator::type_for());

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
            Generator::new(output_dir, &entry.path())?.run(&templates)?;
            break;
        }}}
    }

    Ok(())
}
