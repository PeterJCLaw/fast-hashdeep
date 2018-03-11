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
use std::time::UNIX_EPOCH;

use chrono::naive::NaiveDateTime;
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
    modified: NaiveDateTime,
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


pub fn walk_files(directory: &Path) -> &Iterator<Item = &Path> {
    &WalkDir::new(directory)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
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
            modified: NaiveDateTime::from_timestamp(
                metadata
                    .modified()
                    .unwrap()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
                0,
            ),
            content: ContentDescription {
                size: metadata.len(),
                hash: hash_file(filepath),
            },
            path: filepath,
        }),
    }
}


pub fn path_by_content<'a, I>(descriptions: I) -> HashMap<ContentDescription, PathBuf>
where
    I: IntoIterator<Item = &'a FileDescription>,
{
    HashMap::from_iter(descriptions.into_iter().map(|x| (x.content, x.path)))
}


pub fn load_descriptions<'a, I>(references: I) -> HashMap<PathBuf, FileDescription>
where
    I: IntoIterator<Item = &'a PathBuf>,
{
    HashMap::from_iter(
        references
            .into_iter()
            .flat_map(|path| {
                BufReader::new(File::open(path).unwrap()).lines().map(
                    |line| {
                        FileDescription::parse(line.unwrap().as_str(), path.parent().unwrap())
                    },
                )
            })
            .map(|x| (x.path, x)),
    )
}
