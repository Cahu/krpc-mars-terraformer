use std::collections::HashMap;
use std::path::Path;

mod generator;
use generator::Generator;

mod service_file;
use service_file::ServiceFile;

pub fn run<P, I, U>(services_paths: I, output_dir: U) -> Result<(), Box<dyn std::error::Error>>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
    U: AsRef<Path>,
{
    // Compile templates
    let mut templates = tera::Tera::default();
    templates
        .add_raw_template("macros.tera", include_str!("../templates/macros.tera"))
        .unwrap();
    templates
        .add_raw_template("service.rs", include_str!("../templates/service.rs.tera"))
        .unwrap();

    templates.register_filter("format_doc", format_doc);
    templates.register_filter("format_proc_name", format_proc_name);

    let generator = Generator::new(output_dir.as_ref().to_path_buf(), templates);

    for file_path in services_paths {
        let service_file = ServiceFile::load_from_file(file_path)?;
        generator.run(&service_file)?;
    }

    Ok(())
}

fn format_doc(
    val: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    match val {
        tera::Value::String(s) => {
            let s = s.replace("\n", " ");
            Ok(tera::Value::String(s))
        }
        _ => Err(tera::Error::call_filter("format_doc", "Not a string")),
    }
}

fn format_proc_name(
    val: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    use heck::ToSnakeCase;

    match val {
        tera::Value::String(s) => {
            let s = s.to_snake_case();
            Ok(tera::Value::String(s))
        }
        _ => Err(tera::Error::call_filter("format_proc_name", "Not a string")),
    }
}
