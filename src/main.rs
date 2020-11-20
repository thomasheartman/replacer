use handlebars::Handlebars;
use log::{debug, error, info};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::{collections::HashMap, fmt};
use std::{
    fs::{File, OpenOptions},
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
    output_dir: String,
}

#[derive(Debug)]
enum ProgramError {
    FileNotFound(PathBuf),
    ReadFailed(PathBuf),
    MissingKey(String),
    CannotOpenFileForWriting(PathBuf),
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

fn go(opts: &Opt) -> Result<(), ProgramError> {
    let mut input = open_file(&opts.input_file)?;
    let filename = opts.input_file.as_path().file_name().unwrap();

    let replacements: HashMap<String, String> = deserialize(&opts.replacements_file)?;

    let conf: Config = deserialize(&opts.config_file)?;

    let output = Path::new(&conf.output_dir).join(&filename);

    info!(
        "Creating file {:?} using {:?} as a template and {:?} as replacements file.",
        &output, &opts.input_file, &opts.replacements_file,
    );

    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);

    // replace content
    handlebars
        .render_template_source_to_write(
            &mut input,
            &replacements,
            OpenOptions::new()
                .write(true)
                .open(output.clone())
                .map_err(|_| ProgramError::CannotOpenFileForWriting(output))?,
        )
        .map_err(|_| ProgramError::MissingKey("nope".to_owned()))?;

    Ok(())
}

fn main() -> Result<(), ProgramError> {
    env_logger::init();
    // read input
    let opt = Opt::from_args();
    debug!("Got these options: {:#?}", opt);

    let result = go(&opt);

    match result {
        Ok(_) => {}
        Err(e) => error!("Encountered an error during execution: {}", e),
    }

    // write

    println!("Hello, world!");
    Ok(())
}
