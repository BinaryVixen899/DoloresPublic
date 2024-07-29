//! ```cargo
//! [dependencies]
//! fs_extra  = "*"
//! sudo2 = "0.2.1"
//! ```
use fs_extra::dir::{self, copy, *};
use fs_extra::error::*;
use std::path::Path;
use std::process::exit;

// Basic build script that only works for linux
// PLEASE DO NOT USE THIS AN ACTUAL BUILD SCRIPT.
// ADD build = true TO EDIT, AND SET TO FALSE WHEN YOU ARE DONE EDITING
// THIS SCRIPT VIOLATES THE BUILD SCRIPT SPEC AND SHOULD ONLY BE USED BY MAKEFILE.TOML

// Constants
// TODO: Pass these in via hardcoded env variables in Makefile.toml or have constants.rs constructed via Makefile.toml
const SERINA_CONFIG_PATH: &str = "/etc/serina";

fn main() {
    let options = CopyOptions::new();

    match Path::new(SERINA_CONFIG_PATH).is_dir() {
        true => {
            return println!(
                "Existing serina installation!\nThis is probably an update!\nDoing nothing!"
            );
        }
        false => {
            println!("New serina installation!");
            sudo2::escalate_if_needed().expect("Given sudo because I need it!");
            dir::create(SERINA_CONFIG_PATH, false).unwrap();
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
}
