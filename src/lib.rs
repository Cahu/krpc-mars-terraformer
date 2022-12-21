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

    // An array of our services' names to generate the mod.rs file
    let mut services = Vec::new();

    // Generate rust code for each service file
    for file_path in services_paths {
        let service_file = ServiceFile::load_from_file(file_path)?;
        generator.run(&service_file)?;
        services.extend(service_file.services.into_keys())
    }

    // Finally, generate the mod.rs file
    let mut mod_path = output_dir.as_ref().to_path_buf();
    mod_path.push("mod.rs");

    let mut f = std::fs::File::create(&mod_path)?;
    for s in services {
        use heck::ToSnakeCase;
        use std::io::Write;
        write!(f, "pub mod {};\n", s.to_snake_case())?;
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
