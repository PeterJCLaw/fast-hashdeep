use std::path::PathBuf;
use std::vec::Vec;

use common::describe;
use common::MaybeFileDescription;
use common::walk_files;

pub fn record(directory: PathBuf) {
    for filepath in walk_files(directory) {
        match describe(filepath) {
            MaybeFileDescription::FileDescription(description) => {
                println!("{}", description.to_string());
            }
            MaybeFileDescription::MissingFile(description) => {
                panic!("{:?}", description);
            }
        }
    }
}

pub fn audit(directory: PathBuf, references: Vec<PathBuf>) {
    println!("audit");
}

pub fn compare(baseline: PathBuf, target: PathBuf) {
    println!("compare");
}

pub fn find_duplicates(references: Vec<PathBuf>) {
    println!("find_duplicates");
}
