mod writer;

use env_logger::Env;
use log::{error, info, warn};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::File,
    path::{Path, PathBuf},
};
use structopt::StructOpt;
use writer::{render, Configuration, ProgramError};

#[derive(StructOpt, Debug)]
#[structopt(name = "replacer")]
struct Opts {
    // A file containing a templated text using the Handlebars format
    #[structopt(short = "f", long = "file", parse(from_os_str))]
    input_file: PathBuf,

    // A YAML file containing of key value pairs to be replaced.
    #[structopt(short = "i", parse(from_os_str))]
    replacements_file: PathBuf,

    // A YAML file containing program configuration.
    #[structopt(short = "c", long = "config-file", parse(from_os_str))]
    config_file: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Config {
    output_dir: PathBuf,
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

fn parse_input_files(opts: &Opts) -> Result<Configuration, ProgramError> {
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

fn main() -> Result<(), ()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // read input
    let opts = Opts::from_args();

    let result = parse_input_files(&opts).and_then(render);

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
