use std::borrow::ToOwned;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::fs::Metadata;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::iter::FromIterator;
use std::iter::Iterator;
use std::path::Path;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use std::vec::Vec;

use chrono::naive::NaiveDateTime;
use md5;
use walkdir::WalkDir;

const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.6f";
const HASH_PREFIX_SIZE: usize = 1024 * 1024;

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub struct ContentDescription {
    size: u64,
    hash: String,
}

impl ContentDescription {
    pub fn size(&self) -> u64 {
        self.size
    }
    pub fn hash(&self) -> &str {
        &self.hash
    }
}

#[derive(Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct MovedFile {
    old: PathBuf,
    new: PathBuf,
}

#[derive(Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct CopiedFile {
    old: PathBuf,
    new: PathBuf,
}

#[derive(Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct NewFile {
    path: PathBuf,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct ChangedFile {
    path: PathBuf,
    old_content: ContentDescription,
    new_content: ContentDescription,
}

impl Ord for ChangedFile {
    fn cmp(&self, other: &ChangedFile) -> Ordering {
        self.path.cmp(&other.path)
    }
}

impl PartialOrd for ChangedFile {
    fn partial_cmp(&self, other: &ChangedFile) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct MissingFile {
    path: PathBuf,
}

impl MissingFile {
    pub fn new<P>(filepath: P) -> MissingFile
    where
        P: AsRef<Path>,
    {
        MissingFile {
            path: filepath.as_ref().to_path_buf(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct FileDescription {
    modified: NaiveDateTime,
    content: ContentDescription,
    path: PathBuf,
}

impl FileDescription {
    pub fn content_ref(&self) -> &ContentDescription {
        &self.content
    }
}

impl fmt::Display for FileDescription {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{},{},{},{}",
            self.modified.format(DATE_FORMAT),
            self.content.size,
            self.content.hash,
            self.path.to_str().unwrap(),
        )
    }
}

impl Ord for FileDescription {
    fn cmp(&self, other: &FileDescription) -> Ordering {
        self.path.cmp(&other.path)
    }
}

impl PartialOrd for FileDescription {
    fn partial_cmp(&self, other: &FileDescription) -> Option<Ordering> {
        Some(self.cmp(other))
    }
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
            modified: NaiveDateTime::parse_from_str(parts[0], DATE_FORMAT)
                .expect(&format!("Bad datetime '{}'", parts[0])),
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

impl ChangeSummary {
    pub fn has_changes(&self) -> bool {
        self.changed.len() > 0
            || self.copied.len() > 0
            || self.moved.len() > 0
            || self.deleted.len() > 0
            || self.added.len() > 0
    }

    fn descriptions<'a, T, F>(items: &Vec<T>, title: &'a str, item_formatter: F) -> Option<String>
    where
        T: Ord,
        F: Fn(&T) -> String,
    {
        if items.len() == 0 {
            return None;
        }

        let mut items_clone: Vec<&T> = items.iter().collect();
        items_clone.sort();

        let items_descriptions: Vec<String> = items_clone.into_iter().map(item_formatter).collect();
        let items_description: String = items_descriptions.join("\n");
        Some(format!("# {}:\n{}", title, items_description))
    }

    pub fn describe(&self) -> String {
        let changed_descriptions =
            ChangeSummary::descriptions(&self.changed, "Changed files", |x| {
                format!("{}", x.path.display())
            });
        let copied_descriptions = ChangeSummary::descriptions(&self.copied, "Copied files", |x| {
            format!("{} (from {})", x.new.display(), x.old.display())
        });
        let moved_descriptions = ChangeSummary::descriptions(&self.moved, "Moved files", |x| {
            format!("{} (from {})", x.new.display(), x.old.display())
        });
        let deleted_descriptions =
            ChangeSummary::descriptions(&self.deleted, "Deleted files", |x| {
                format!("{}", x.path.display())
            });
        let added_descriptions =
            ChangeSummary::descriptions(&self.added, "Added files", |x| format!("{}", x));

        let descriptions: Vec<String> = vec![
            changed_descriptions,
            copied_descriptions,
            moved_descriptions,
            deleted_descriptions,
            added_descriptions,
        ]
        .into_iter()
        .filter_map(|x| x)
        .collect();
        descriptions.join("\n")
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum MaybeFileDescription {
    MissingFile(MissingFile),
    FileDescription(FileDescription),
}

// TODO: this would ideally return `-> impl Iterator<Item = PathBuf>`, but
// that's not stable yet.
pub fn walk_files<P>(directory: P) -> Vec<PathBuf>
where
    P: AsRef<Path>,
{
    WalkDir::new(directory)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_type().is_dir())
        .map(|e| e.path().to_path_buf())
        .collect()
}

pub fn hash_file<P>(filepath: P) -> String
where
    P: AsRef<Path>,
{
    let mut f = File::open(filepath).unwrap();
    let mut buffer = [0; HASH_PREFIX_SIZE];
    let read_size: usize = f.read(&mut buffer).unwrap();
    let digest = md5::compute(&buffer[..read_size]);
    format!("{:x}", digest)
}

pub fn describe(filepath: PathBuf) -> MaybeFileDescription {
    fn from_metadata(metadata: &Metadata) -> NaiveDateTime {
        let timestamp = metadata
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap();
        NaiveDateTime::from_timestamp_opt(timestamp.as_secs() as i64, timestamp.subsec_nanos())
            .unwrap()
    }

    let metadata_result = filepath.metadata();
    match metadata_result {
        Err(_) => MaybeFileDescription::MissingFile(MissingFile { path: filepath }),
        Ok(metadata) => MaybeFileDescription::FileDescription(FileDescription {
            modified: from_metadata(&metadata),
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
    HashMap::from_iter(
        descriptions
            .into_iter()
            .map(|x| (&x.content, x.path.as_path())),
    )
}

pub fn load_descriptions<'a, I, P>(references: I) -> HashMap<PathBuf, FileDescription>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    HashMap::from_iter(
        references
            .into_iter()
            .flat_map(|path| {
                let reader = BufReader::new(File::open(path.as_ref()).unwrap());
                let parent = path.as_ref().parent().unwrap();
                let descriptions: Vec<FileDescription> = reader
                    .lines()
                    .map(|line| FileDescription::parse(&line.unwrap(), parent))
                    .collect();
                descriptions
            })
            .map(|x| (x.path.clone(), x)),
    )
}

pub fn describe_differences(
    expected: &HashMap<PathBuf, FileDescription>,
    current: &HashMap<PathBuf, MaybeFileDescription>,
) -> ChangeSummary {
    let mut missing: Vec<&Path> = Vec::new();
    let mut actual: HashMap<&Path, &FileDescription> = HashMap::new();
    let mut unexpected: HashMap<&Path, &FileDescription> = HashMap::new();

    let mut changed: Vec<ChangedFile> = Vec::new();

    for (filepath, maybe_description) in current {
        match maybe_description {
            &MaybeFileDescription::MissingFile(_) => missing.push(filepath.as_path()),
            &MaybeFileDescription::FileDescription(ref description) => {
                actual.insert(filepath.as_path(), &description);
                match expected.get(filepath) {
                    None => {
                        unexpected.insert(filepath.as_path(), &description);
                    }
                    Some(expected_description) => {
                        if expected_description != description {
                            changed.push(ChangedFile {
                                path: filepath.clone(),
                                old_content: expected_description.content.clone(),
                                new_content: description.content.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    let path_by_expected_content = path_by_content(expected.values());
    let current_descriptions: Vec<&FileDescription> = current
        .values()
        .filter_map(|x| match x {
            &MaybeFileDescription::MissingFile(_) => None,
            &MaybeFileDescription::FileDescription(ref description) => Some(description),
        })
        .collect();
    let path_by_actual_content = path_by_content(current_descriptions);

    let mut copied: Vec<CopiedFile> = Vec::new();
    let mut moved: Vec<MovedFile> = Vec::new();
    let mut deleted: Vec<MissingFile> = Vec::new();
    let mut new_files: Vec<FileDescription> = Vec::new();

    for missing_path in missing {
        let missing_path_buf = missing_path.to_path_buf();
        let expected_content = &expected.get(&missing_path_buf).unwrap().content;
        match path_by_actual_content.get(expected_content) {
            Some(new_path) => moved.push(MovedFile {
                old: missing_path_buf,
                new: new_path.to_path_buf(),
            }),
            None => deleted.push(MissingFile {
                path: missing_path_buf,
            }),
        }
    }

    for (filepath, description) in unexpected {
        match path_by_expected_content.get(&description.content) {
            Some(expected_path) => {
                if actual.contains_key(expected_path) {
                    copied.push(CopiedFile {
                        old: expected_path.to_path_buf(),
                        new: filepath.to_path_buf(),
                    });
                }
            }
            None => new_files.push(description.clone()),
        }
    }

    ChangeSummary {
        changed: changed,
        copied: copied,
        moved: moved,
        deleted: deleted,
        added: new_files,
    }
}
