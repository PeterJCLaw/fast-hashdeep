use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::BufRead;
use std::iter::FromIterator;
use std::iter::Iterator;
use std::path::Path;
use std::vec::Vec;
use std::time::SystemTime;

use md5;
use walkdir::WalkDir;

const HASH_PREFIX_SIZE: usize = 1024 * 1024;

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct ContentDescription<'a> {
    size: u64,
    hash: &'a str,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct MovedFile<'a> {
    old: &'a Path,
    new: &'a Path,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct CopiedFile<'a> {
    old: &'a Path,
    new: &'a Path,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct NewFile<'a> {
    path: &'a Path,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct ChangedFile<'a> {
    path: &'a Path,
    old_content: ContentDescription<'a>,
    new_content: ContentDescription<'a>,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct MissingFile<'a> {
    path: &'a Path,
}

#[derive(Debug, Eq, PartialEq)]
pub struct FileDescription<'a> {
    modified: SystemTime,
    content: ContentDescription<'a>,
    path: &'a Path,
}

#[derive(Debug)]
pub struct ChangeSummary<'a> {
    changed: Vec<ChangedFile<'a>>,
    copied: Vec<CopiedFile<'a>>,
    moved: Vec<MovedFile<'a>>,
    deleted: Vec<MissingFile<'a>>,
    added: Vec<FileDescription<'a>>,
}


#[derive(Debug, Eq, PartialEq)]
pub enum MaybeFileDescription<'a> {
    MissingFile(MissingFile<'a>),
    FileDescription(FileDescription<'a>),
}


pub fn walk_files(directory: Path) -> Iterator<Item=Path> {
    WalkDir::new(directory)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
}


pub fn hash_file(filepath: Path) -> str {
    let f = File::open(filepath);
    let mut buffer = [0; HASH_PREFIX_SIZE];
    f.read_exact(&mut buffer);
    let digest = md5::compute(&buffer[..]);
    format!("{:x}", digest)
}


pub fn describe<'a>(filepath: Path) -> MaybeFileDescription<'a> {
    let metadata_result = filepath.metadata();
    match metadata_result {
        Err(_) => MaybeFileDescription::MissingFile(MissingFile { path: filepath }),
        Ok(metadata) => MaybeFileDescription::FileDescription(FileDescription {
            modified: metadata.modified().unwrap(),
            content: ContentDescription {
                size: metadata.len(),
                hash: hash_file(filepath),
            },
            path: filepath,
        }),
    }
}


pub fn path_by_content(
    descriptions: Iterator<Item=FileDescription>,
) -> HashMap<ContentDescription, Path> {
    HashMap::from_iter(descriptions.map(|x| (x.content, x.path)))
}


pub fn load_descriptions<'a>(references: Iterator<Item=Path>) -> HashMap<Path, FileDescription<'a>> {
    HashMap::from_iter(
        references
            .flat_map(|path| {
                BufReader::new(File::open(path).unwrap()).lines().map(|line| {
                    FileDescription::parse(line, path.parent())
                })
            })
            .map(|x| (x.content, x.path)),
    )
}
