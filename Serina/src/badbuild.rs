//! ```cargo
//! [dependencies]
//! fs_extra  = "*"
//! ```
use std::env;
use std::fs::create_dir;
use std::path::Path;
use std::process::exit;

// Basic build script that only works for linux

// use constants::{PHRASES_CONFIG_PATH, SERINA_CONFIG_PATH};
use fs_extra::dir::{copy, *};
use fs_extra::error::*;
use std::fs;

// please note that the following is very hacky and the actual way to do it is to separate out the needed types and such.
// But screw that
// const cwd: &str = env::var_os("CARGO_MAKE_WORKING_DIRECTORY").unwrap();
// const cwpath: &str = cwd + "/src/constants.rs";

// Constants
// TODO: Pass these in via hardcoded env variables in Makefile.toml
// TODO: have constants.rs constructed via Makefile.toml
const HEADER_PREFIX: &str = "OTEL_EXPORTER_";
const CONFIG_PATH: &str = "/etc/serina/.env";
const PHRASES_CONFIG_PATH: &str = "/etc/serina/phrases.txt";
const SERINA_CONFIG_PATH: &str = "/etc/serina";

fn main() {
    let options = CopyOptions::new();

    match Path::new(SERINA_CONFIG_PATH).try_exists() {
        Ok(_) => {
            return println!("Existing serina installation!");
            println!("This must be an update!")
        }
        Err(_) => {
            println!("New serina installation!");
        }
    };
    println!("Creating /etc/serina directory and copying example files!");
    let copy = copy("./template", SERINA_CONFIG_PATH, &options);
    if let Err(e) = copy.map_err(|e| e.kind) {
        match e {
            ErrorKind::AlreadyExists => {
                eprintln!("/etc/serina already exists, how did you get here?!?");
                eprintln!("Actual error was {:#?}", e);
            }
            ErrorKind::NotFound => {
                eprintln!("Something really weird happened");
                eprintln!("Actual error was {:#?}", e);
            }
            ErrorKind::PermissionDenied => {
                eprintln!("Permission denied. Are you sure you have permission to read and write under /etc/?");
                eprintln!("Actual error was {:#?}", e);
            }
            ErrorKind::Other => {
                eprintln!("I have no clue.");
                eprintln!("Actual error was {:#?}", e);
            }
            _ => {
                eprintln!("Some weird error occurred");
            }
        }
        exit(1);
    };
    println!("Copied example files to directory");
    println!("Proceeding to installation!")

    // Check if there is already a .env file or phrases_example.txt
    // If this is the case, bail out
}
