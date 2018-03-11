use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::BufReader;
use std::io::BufRead;
use std::iter::FromIterator;
use std::iter::Iterator;
use std::path::PathBuf;
use std::path::Path;
use std::vec::Vec;
use std::time::SystemTime;

use md5;
use walkdir::WalkDir;

const HASH_PREFIX_SIZE: usize = 1024 * 1024;

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct ContentDescription {
    size: u64,
    hash: String,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct MovedFile {
    old: PathBuf,
    new: PathBuf,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct CopiedFile {
    old: PathBuf,
    new: PathBuf,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct NewFile {
    path: PathBuf,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct ChangedFile {
    path: PathBuf,
    old_content: ContentDescription,
    new_content: ContentDescription,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct MissingFile {
    path: PathBuf,
}

#[derive(Debug, Eq, PartialEq)]
pub struct FileDescription {
    modified: SystemTime,
    content: ContentDescription,
    path: PathBuf,
}

#[derive(Debug)]
pub struct ChangeSummary {
    changed: Vec<ChangedFile>,
    copied: Vec<CopiedFile>,
    moved: Vec<MovedFile>,
    deleted: Vec<MissingFile>,
    added: Vec<FileDescription>,
}


#[derive(Debug, Eq, PartialEq)]
pub enum MaybeFileDescription {
    MissingFile(MissingFile),
    FileDescription(FileDescription),
}


pub fn walk_files(directory: &Path) -> &Iterator<Item = PathBuf> {
    WalkDir::new(directory).into_iter().filter_map(|e| e.ok())
}


pub fn hash_file(filepath: PathBuf) -> String {
    let f = File::open(filepath).unwrap();
    let mut buffer = [0; HASH_PREFIX_SIZE];
    f.read_exact(&mut buffer);
    let digest = md5::compute(&buffer[..]);
    format!("{:x}", digest)
}


pub fn describe(filepath: PathBuf) -> MaybeFileDescription {
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
    descriptions: &IntoIterator<Item = FileDescription, IntoIter = Iterator<Item = FileDescription>>,
) -> HashMap<ContentDescription, PathBuf> {
    HashMap::from_iter(descriptions.map(|x| (x.content, x.path)))
}


pub fn load_descriptions(
    references: &IntoIterator<Item = PathBuf, IntoIter = Iterator<Item = PathBuf>>,
) -> HashMap<PathBuf, FileDescription> {
    HashMap::from_iter(
        references
            .flat_map(|path| {
                BufReader::new(File::open(path).unwrap()).lines().map(
                    |line| {
                        FileDescription::parse(line, path.parent())
                    },
                )
            })
            .map(|x| (x.content, x.path)),
    )
}
