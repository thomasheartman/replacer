use handlebars::Handlebars;
use log::{debug, info};
use serde::Deserialize;
use std::collections::HashMap;
use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
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

fn main() -> anyhow::Result<()> {
    env_logger::init();
    // read input
    let opt = Opt::from_args();
    debug!("Got these options: {:#?}", opt);

    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);

    let mut input = File::open(&opt.input_file)?;

    let replacements: HashMap<String, String> = File::open(&opt.replacements_file)
        .map(serde_yaml::from_reader::<_, HashMap<String, String>>)??;

    let conf = File::open(opt.config_file).map(serde_yaml::from_reader::<_, Config>)??;

    info!(
        "Creating file {:?} using {:?} as a template and {:?} as replacements file.",
        &conf.output_dir, &opt.input_file, &opt.replacements_file,
    );

    // replace content
    let output = handlebars.render_template_source_to_write(
        &mut input,
        &replacements,
        OpenOptions::new().write(true).open(conf.output_dir)?,
    )?;

    // write

    println!("Hello, world!");
    Ok(())
}
