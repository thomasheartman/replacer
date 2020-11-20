use handlebars::Handlebars;
use log::{debug, error, info, warn};
use serde::de::DeserializeOwned;
use serde::Deserialize;
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
    #[structopt(short = "c", long = "configuration", parse(from_os_str))]
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
    MissingKey(String),
    CannotOpenFileForWriting(PathBuf),
    CannotCreateOutputDirectories(PathBuf),
}

impl fmt::Display for ProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProgramError::FileNotFound(path) => write!(f, "Couldn't find the file {:?}.", path),
            ProgramError::ReadFailed(path) => write!(
                f,
                "Couldn't deserialize {:?} into the expected format.",
                path
            ),
            ProgramError::MissingKey(key) => {
                write!(f, "The key '{}' does not have a replacement.", key)
            }
            ProgramError::CannotOpenFileForWriting(path) => {
                write!(f, "Couldn't open output file {:?}.", path)
            }
            ProgramError::CannotCreateOutputDirectories(path) => {
                write!(f, "Failed to create directory {:?}", path)
            }
        }
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
    let mut input = open_file(&opts.input_file)?;
    let filename = opts.input_file.as_path().file_name().unwrap_or_else(|| {
        let default_name = "output";
        warn!(
            "Unable to generate an output filename based on the the input file; using {} instead.",
            &default_name
        );
        OsStr::new(default_name)
    });

    let replacements: HashMap<String, String> = deserialize(&opts.replacements_file)?;

    let conf: Config = deserialize(&opts.config_file)?;

    let output = Path::new(&conf.output_dir).join(&filename);

    info!(
        "Creating file {:?} using {:?} as a template and {:?} as a replacements file.",
        &output, &opts.input_file, &opts.replacements_file,
    );

    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);

    info!("Creating necessary directories.");
    DirBuilder::new()
        .recursive(true)
        .create(&conf.output_dir)
        .map_err(|_| ProgramError::CannotCreateOutputDirectories(conf.output_dir))?;

    handlebars
        .render_template_source_to_write(
            &mut input,
            &replacements,
            OpenOptions::new()
                .create(true)
                .write(true)
                .open(&output)
                .map_err(|_| ProgramError::CannotOpenFileForWriting(output.clone()))?,
        )
        .map_err(|_| ProgramError::MissingKey("nope".to_owned()))?;

    Ok(output)
}

fn main() -> Result<(), ()> {
    env_logger::init();
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
