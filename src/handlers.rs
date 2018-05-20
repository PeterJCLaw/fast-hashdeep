use std::collections::HashMap;
use std::iter::FromIterator;
use std::iter::Iterator;
use std::path::PathBuf;
use std::vec::Vec;

use common::describe;
use common::describe_differences;
use common::load_descriptions;
use common::MaybeFileDescription;
use common::MissingFile;
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
    let expected = load_descriptions(references);
    let mut current = HashMap::from_iter(expected.keys().map(|p| {
        (p.to_path_buf(), describe(p.to_path_buf()))
    }));

    for filepath in walk_files(directory) {
        current.entry(filepath.clone()).or_insert_with(
            || describe(filepath),
        );
    }
    let change_summary = describe_differences(&expected, &current);

    if change_summary.has_changes() {
        println!("{}", change_summary.describe())
    }
}

pub fn compare(baseline: PathBuf, target: PathBuf) {
    let baseline_descriptions = load_descriptions(vec![baseline]);
    let mut target_descriptions: HashMap<PathBuf, MaybeFileDescription> =
        HashMap::from_iter(load_descriptions(vec![target]).into_iter().map(|(k, v)| {
            (k, MaybeFileDescription::FileDescription(v))
        }));

    for filepath in baseline_descriptions.keys() {
        target_descriptions.entry(filepath.clone()).or_insert(
            MaybeFileDescription::MissingFile(MissingFile::new(filepath)),
        );
    }

    let change_summary = describe_differences(&baseline_descriptions, &target_descriptions);

    if change_summary.has_changes() {
        println!("{}", change_summary.describe())
    }
}

pub fn find_duplicates(references: Vec<PathBuf>) {
    println!("find_duplicates");
}
