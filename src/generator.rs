use std::path::PathBuf;

use std::collections::BTreeMap;

use heck::ToSnakeCase;

use crate::service_file::Service;
use crate::service_file::ServiceFile;

pub struct Generator {
    output_dir: PathBuf,
    templates: tera::Tera,
}

#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error(transparent)]
    IOErr(#[from] std::io::Error),
    #[error(transparent)]
    TeraErr(#[from] tera::Error),
}

impl Generator {
    pub fn new(output_dir: PathBuf, templates: tera::Tera) -> Self {
        Generator {
            output_dir,
            templates,
        }
    }

    /// Generate rust code for the given service file
    pub fn run(&self, service_file: &ServiceFile) -> Result<(), GenerateError> {
        for (service_name, service_defs) in &service_file.services {
            let (classes, procedures) = separate_procedures_from_methods(service_defs);

            let mut context = tera::Context::new();
            context.insert("service_name", &service_name);
            context.insert("service_deps", &service_file.get_deps());
            context.insert("service_classes", &classes);
            context.insert("service_procedures", &procedures);
            context.insert("service_enumerations", &service_defs.enumerations);

            // Build the path to the output file
            let mut output_file_path = self.output_dir.clone();
            output_file_path.push(service_name.to_snake_case());
            output_file_path.set_extension("rs");

            // Generate the file
            let output = std::fs::File::create(output_file_path)?;
            self.templates.render_to("service.rs", &context, output)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct GenProcedure {
    pub documentation: String,
    pub parameters: Vec<GenProcParameter>,
    pub return_type: String,
    pub return_is_nullable: bool,
}

impl From<crate::service_file::Procedure> for GenProcedure {
    fn from(proc: crate::service_file::Procedure) -> Self {
        let return_type = proc
            .return_type
            .as_ref()
            .map(crate::service_file::Type::to_rust_type)
            .unwrap_or("()".to_string());
        let parameters = proc
            .parameters
            .into_iter()
            .map(GenProcParameter::from)
            .collect();
        Self {
            documentation: proc.documentation,
            parameters,
            return_type,
            return_is_nullable: proc.return_is_nullable,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct GenProcParameter {
    pub name: String,
    pub r#type: String,
}

impl From<crate::service_file::ProcParameter> for GenProcParameter {
    fn from(param: crate::service_file::ProcParameter) -> Self {
        Self {
            name: param.name.to_snake_case(),
            r#type: param.r#type.to_rust_type(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct GenClass {
    /// The documentation for the class
    documentation: String,
    /// Methods associated with the class
    methods: BTreeMap<String, GenProcedure>,
}

type ClassesMap = BTreeMap<String, GenClass>;
type ProceduresMap = BTreeMap<String, GenProcedure>;

/// Methods name are of the form `Class_MethodName` in the json file. Recognize them
/// and separate them from free functions.
fn separate_procedures_from_methods(service: &Service) -> (ClassesMap, ProceduresMap) {
    let mut classes = ClassesMap::new();
    let mut procedures = ProceduresMap::new();
    for (proc_name, proc_def) in &service.procedures {
        if let Some((class_name, method_name)) = proc_name.split_once('_') {
            if let Some(service_class) = service.classes.get(class_name) {
                // This is a method
                let class = classes
                    .entry(class_name.to_string())
                    .or_insert_with(|| GenClass {
                        documentation: service_class.documentation.clone(),
                        methods: BTreeMap::new(),
                    });
                // The first param of every method is 'this'. Remove it.
                let mut proc_def = proc_def.clone();
                proc_def.parameters.remove(0);
                class
                    .methods
                    .insert(method_name.to_string(), proc_def.into());
            } else {
                // This is a procedures
                procedures.insert(proc_name.clone(), proc_def.clone().into());
            }
        }
    }
    (classes, procedures)
}
