use std::path::PathBuf;
use std::vec::Vec;

use common::walk_files;
use common::describe;

pub fn record(directory: PathBuf) {
    for filepath in walk_files(directory) {
        println!("{:?}", describe(filepath));
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
