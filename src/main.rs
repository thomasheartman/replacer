use env_logger::Env;
use handlebars::{Handlebars, TemplateFileError};
use log::{debug, error, info, warn};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::io::Write;
use std::{collections::HashMap, fmt};
use std::{
    ffi::OsStr,
    fs::{DirBuilder, File, OpenOptions},
    path::{Path, PathBuf},
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "replacer")]
struct Opt {
    // A file containing a templated text using the Handlebars format
    #[structopt(short = "f", long = "file", parse(from_os_str))]
    input_file: PathBuf,

    // A map of values to be replaced and their replacements in YAML
    #[structopt(short = "i", parse(from_os_str))]
    replacements_file: PathBuf,

    // A configuration file, YAML
    #[structopt(short = "c", long = "config-file", parse(from_os_str))]
    config_file: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Config {
    output_dir: PathBuf,
}

#[derive(Debug)]
enum ProgramError {
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

fn open_file(path: &PathBuf) -> Result<File, ProgramError> {
    File::open(path).map_err(|_| ProgramError::FileNotFound(path.clone()))
}

fn deserialize<T>(path: &PathBuf) -> Result<T, ProgramError>
where
    T: DeserializeOwned,
{
    open_file(path).and_then(|f| {
        serde_yaml::from_reader::<_, T>(f).map_err(|_| ProgramError::ReadFailed(path.clone()))
    })
}

fn go(opts: &Opt) -> Result<PathBuf, ProgramError> {
    struct Configuration {
        template: File,
        mappings: HashMap<String, String>,
        output_file: PathBuf,
    }
    fn parse_input_files(opts: &Opt) -> Result<Configuration, ProgramError> {
        let template = open_file(&opts.input_file)?;
        let mappings: HashMap<String, String> = deserialize(&opts.replacements_file)?;
        let config: Config = deserialize(&opts.config_file)?;
        let filename = opts.input_file.as_path().file_name().unwrap_or_else(|| {
            let default_name = "output";
            warn!(
            "Unable to generate an output filename based on the the input file; using {} instead.",
            &default_name
        );
            OsStr::new(default_name)
        });

        let output_file = Path::new(&config.output_dir.join(&filename)).to_path_buf();

        info!(
            "Creating file {:?} using {:?} as a template and {:?} as a replacements file.",
            &output_file, &opts.input_file, &opts.replacements_file,
        );

        Ok(Configuration {
            template,
            mappings,
            output_file,
        })
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
                .map_err(|_| {
                    ProgramError::CannotCreateOutputDirectories(parent_dir.to_path_buf())
                })?;
        };

        OpenOptions::new()
            .create(true)
            .write(true)
            .open(&output_file)
            .and_then(|mut f| f.write(&result.as_bytes()))
            .map_err(|_| ProgramError::CannotOpenFileForWriting(output_file.clone()))?;

        Ok(output_file)
    }
    parse_input_files(opts)
        .and_then(render_template)
        .and_then(write_template_file)
}

fn main() -> Result<(), ()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // read input
    let opt = Opt::from_args();
    debug!("Got these options: {:#?}", opt);

    let result = go(&opt);

    match result {
        Ok(path) => {
            info!("Successfully wrote file {:?}", path);
            Ok(())
        }
        Err(e) => {
            error!("Encountered an error during execution: {}", e);
            Err(())
        }
    }
}
