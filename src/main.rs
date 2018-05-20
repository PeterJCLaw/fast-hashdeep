#[macro_use]
extern crate structopt;

extern crate chrono;
extern crate itertools;
extern crate md5;
extern crate walkdir;

use std::path::PathBuf;
use std::vec::Vec;
use structopt::StructOpt;

mod common;
mod handlers;
use handlers::{record, audit, compare, find_duplicates};

#[derive(Debug, StructOpt)]
#[structopt(name = "fash-hashdeep")]
enum Opt {
    #[structopt(name = "record")]
    /// Record the current state of the directory
    Record {
        #[structopt(parse(from_os_str))]
        directory: PathBuf,
    },

    #[structopt(name = "audit")]
    /// Audit records in the given files
    Audit {
        #[structopt(parse(from_os_str))]
        directory: PathBuf,

        #[structopt(parse(from_os_str))]
        references: Vec<PathBuf>,
    },

    #[structopt(name = "compare")]
    /// Compare records in the given files
    Compare {
        #[structopt(parse(from_os_str))]
        baseline: PathBuf,

        #[structopt(parse(from_os_str))]
        target: PathBuf,
    },

    #[structopt(name = "find-duplicates")]
    /// Search for duplicates within the given files
    FindDuplicates {
        #[structopt(parse(from_os_str))]
        references: Vec<PathBuf>,
    },
}

fn main() {
    let matches = Opt::from_args();

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
