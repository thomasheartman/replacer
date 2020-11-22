use handlebars::{Handlebars, TemplateFileError};
use log::info;

use std::io::Write;
use std::{collections::HashMap, fmt};
use std::{
    fs::{DirBuilder, File, OpenOptions},
    path::PathBuf,
};

#[derive(Debug)]
pub(crate) enum ProgramError {
    FileNotFound(PathBuf),
    ReadFailed(PathBuf),
    RenderError(String),
    MissingKey(String),
    InvalidTemplate(String),
    CannotOpenFileForWriting(PathBuf),
    CannotCreateOutputDirectories(PathBuf),
}

impl fmt::Display for ProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            ProgramError::FileNotFound(path) => format!("Couldn't find the file {:?}.", path),
            ProgramError::ReadFailed(path) => {
                format!("Couldn't deserialize {:?} into the expected format.", path)
            }
            ProgramError::RenderError(description) => {
                format!("Template Render error: {}.", description)
            }
            ProgramError::CannotOpenFileForWriting(path) => {
                format!("Couldn't open output file {:?}.", path)
            }
            ProgramError::CannotCreateOutputDirectories(path) => {
                format!("Failed to create directory {:?}", path)
            }
            ProgramError::MissingKey(msg) => msg.clone(),
            ProgramError::InvalidTemplate(reason) => {
                format!(
                    "The provided template file is invalid and cannot be parsed correctly: {}",
                    reason
                )
            }
        };
        write!(f, "{}", msg)
    }
}

pub(crate) struct Configuration {
    pub(crate) template: File,
    pub(crate) mappings: HashMap<String, String>,
    pub(crate) output_file: PathBuf,
}

struct RenderResult {
    result: String,
    output_file: PathBuf,
}

fn render_template(mut config: Configuration) -> Result<RenderResult, ProgramError> {
    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);

    handlebars
        .register_template_source("input", &mut config.template)
        .map_err(|e| match e {
            TemplateFileError::TemplateError(err) => {
                ProgramError::InvalidTemplate(err.reason.to_string())
            }
            TemplateFileError::IOError(_, _) => {
                ProgramError::RenderError(String::from("I/O Error when rendering template."))
            }
        })?;
    handlebars
        .render("input", &config.mappings)
        .map_err(|e| {
            if e.desc.starts_with("Variable") {
                ProgramError::MissingKey(e.desc)
            } else if e.desc.starts_with("Template not found") {
                ProgramError::InvalidTemplate("Couldn't recognize template.".to_string())
            } else {
                ProgramError::RenderError(e.desc)
            }
        })
        .map(|result| RenderResult {
            result,
            output_file: config.output_file,
        })
}

fn write_template_file(
    RenderResult {
        result,
        output_file,
    }: RenderResult,
) -> Result<PathBuf, ProgramError> {
    info!("Creating necessary directories.");

    if let Some(parent_dir) = &output_file.as_path().parent() {
        DirBuilder::new()
            .recursive(true)
            .create(&parent_dir)
            .map_err(|_| ProgramError::CannotCreateOutputDirectories(parent_dir.to_path_buf()))?;
    };

    OpenOptions::new()
        .create(true)
        .write(true)
        .open(&output_file)
        .and_then(|mut f| f.write(&result.as_bytes()))
        .map_err(|_| ProgramError::CannotOpenFileForWriting(output_file.clone()))?;

    Ok(output_file)
}

pub(crate) fn render(config: Configuration) -> Result<PathBuf, ProgramError> {
    render_template(config).and_then(write_template_file)
}
