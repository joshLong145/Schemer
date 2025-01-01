use std::fs;
use clap::{Arg, Command, Parser};

mod common;


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    pub path: String,
}

fn main() {
    pretty_env_logger::init();

    let m = Command::new("rsc")
        .author("Josh Long")
        .version("0.0.0")
        .about("A Scheme implemention")
        .args([Arg::new("path")
            .long("path")
            .alias("path")
            .help("Path to file for interpetetation")])
        .get_matches();

    if let Some(path) = m.get_one::<String>("path") {
        let mut file_buf = fs::read_to_string(path).unwrap();
        file_buf = file_buf.trim().to_string();
        file_buf = file_buf.replace('\n', "");

        crate::common::parse_and_run_scheme(file_buf);
    } else {
        let _ = crate::common::repl();
    }
}


