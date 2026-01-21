use clap::{Arg, Command, Parser};
use std::fs;

mod common;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input file path (positional argument)
    pub path: Option<String>,

    /// Compile to native executable instead of interpreting
    #[arg(short, long)]
    pub compile: bool,

    /// Output file for compiled executable
    #[arg(short, long)]
    pub output: Option<String>,
}

fn main() {
    pretty_env_logger::init();

    let m = Command::new("rsc")
        .author("Josh Long")
        .version("0.0.0")
        .about("A Scheme implementation")
        .args([
            Arg::new("path")
                .help("Path to file for interpretation or compilation")
                .index(1),
            Arg::new("compile")
                .long("compile")
                .short('c')
                .action(clap::ArgAction::SetTrue)
                .help("Compile to native executable instead of interpreting"),
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output file for compiled executable"),
        ])
        .get_matches();

    let compile_mode = m.get_flag("compile");

    if let Some(path) = m.get_one::<String>("path") {
        if compile_mode {
            // Compile mode
            let output = m
                .get_one::<String>("output")
                .map(|s| s.clone())
                .unwrap_or_else(|| {
                    // Default: input.scm -> input (no extension)
                    std::path::Path::new(path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("a.out")
                        .to_string()
                });

            match schemer::compiler::compile_file(path, &output) {
                Ok(()) => println!("Compiled {} -> {}", path, output),
                Err(e) => {
                    eprintln!("Compilation error: {}", e);
                    std::process::exit(1);
                }
            }
        } else {
            // Interpret mode
            let file_buf = fs::read_to_string(path).unwrap();
            crate::common::parse_and_run_scheme(file_buf);
        }
    } else {
        let _ = crate::common::repl();
    }
}
