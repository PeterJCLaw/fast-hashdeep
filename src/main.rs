extern crate chrono;
extern crate clap;
extern crate itertools;
extern crate md5;
extern crate walkdir;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::vec::Vec;

mod common;
mod handlers;
use handlers::{audit, compare, find_duplicates, record};

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Opt,
}

#[derive(Debug, Subcommand)]
#[command(name = "fast-hashdeep")]
enum Opt {
    #[command(name = "record")]
    /// Record the current state of the directory
    Record {
        #[arg()]
        directory: PathBuf,
    },

    #[command(name = "audit")]
    /// Audit records in the given files
    Audit {
        #[arg()]
        directory: PathBuf,

        #[arg()]
        references: Vec<PathBuf>,
    },

    #[command(name = "compare")]
    /// Compare records in the given files
    Compare {
        #[arg()]
        baseline: PathBuf,

        #[arg()]
        target: PathBuf,
    },

    #[command(name = "find-duplicates")]
    /// Search for duplicates within the given files
    FindDuplicates {
        #[arg()]
        references: Vec<PathBuf>,
    },
}

fn main() {
    let matches = Cli::parse().command;

    match matches {
        Opt::Record { directory } => record(directory),
        Opt::Audit {
            directory,
            references,
        } => audit(directory, references),
        Opt::Compare { baseline, target } => compare(baseline, target),
        Opt::FindDuplicates { references } => find_duplicates(references),
    }
}
