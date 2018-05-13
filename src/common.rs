use std::collections::HashMap;
use std::borrow::ToOwned;
use std::fs::File;
use std::io::Read;
use std::io::BufReader;
use std::io::BufRead;
use std::iter::FromIterator;
use std::iter::Iterator;
use std::path::PathBuf;
use std::path::Path;
use std::vec::Vec;
use std::time::UNIX_EPOCH;

use chrono::naive::NaiveDateTime;
use md5;
use walkdir::WalkDir;

const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
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


impl FileDescription {
    pub fn parse(string: &str, relative_to: &Path) -> Self {
        let mut count = 0;
        let maxsplit = 3;
        let parts: Vec<&str> = string
            .trim()
            .split(|c| {
                let should_split = c == ',' && count < maxsplit;
                if should_split {
                    count += 1;
                }
                should_split
            })
            .collect();
        FileDescription {
            modified: NaiveDateTime::parse_from_str(parts[0], DATE_FORMAT).unwrap(),
            content: ContentDescription {
                size: parts[1].parse().unwrap(),
                hash: parts[2].to_owned(),
            },
            path: relative_to.join(parts[3]),
        }
    }
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


// TODO: see if we can introduce a lifetime to allow this to be an iterator?
pub fn walk_files(directory: &Path) -> Vec<PathBuf> {
    WalkDir::new(directory)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .collect()
}


pub fn hash_file<P>(filepath: P) -> String
where
    P: AsRef<Path>,
{
    let mut f = File::open(filepath).unwrap();
    let mut buffer = [0; HASH_PREFIX_SIZE];
    f.read_exact(&mut buffer).unwrap();
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
                hash: hash_file(&filepath),
            },
            path: filepath,
        }),
    }
}


pub fn path_by_content<'a, I>(descriptions: I) -> HashMap<&'a ContentDescription, &'a Path>
where
    I: IntoIterator<Item = &'a FileDescription>,
{
    HashMap::from_iter(descriptions.into_iter().map(
        |x| (&x.content, x.path.as_path()),
    ))
}


pub fn load_descriptions<'a, I>(references: I) -> HashMap<PathBuf, FileDescription>
where
    I: IntoIterator<Item = &'a PathBuf>,
{
    HashMap::from_iter(
        references
            .into_iter()
            .flat_map(|path| {
                let reader = BufReader::new(File::open(path).unwrap());
                let descriptions: Vec<FileDescription> = reader
                    .lines()
                    .map(|line| {
                        FileDescription::parse(&line.unwrap(), path.parent().unwrap())
                    })
                    .collect();
                descriptions
            })
            .map(|x| (x.path.clone(), x)),
    )
}
