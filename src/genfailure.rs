use std::io;
use tera;
use serde_json as json;

#[derive(Debug)]
pub enum GenFailure {
    IoFailure(io::Error),
    JsonFailure(json::Error),
    TemplateFailure(tera::Error),
    ParseError(String),
}
