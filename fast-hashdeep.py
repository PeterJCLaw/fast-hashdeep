#!/usr/bin/env python3.6

import datetime
import hashlib
import os.path
import pathlib
from typing import (
    Dict,
    Iterable,
    Iterator,
    List,
    NamedTuple,
    TextIO,
    Tuple,
    Union
)

import click
import dateutil.parser

HASH_PREFIX_SIZE = 1024 * 1024


class MovedFile(NamedTuple):
    old: pathlib.Path
    new: pathlib.Path


class CopiedFile(NamedTuple):
    old: pathlib.Path
    new: pathlib.Path


class NewFile(NamedTuple):
    path: pathlib.Path


class MissingFile(NamedTuple):
    path: pathlib.Path


class ContentDescription(NamedTuple):
    size: int
    hash: str


class FileDescription(NamedTuple):
    modified: datetime.datetime
    content: ContentDescription
    path: pathlib.Path

    @classmethod
    def create(
        cls,
        *,
        modified: datetime.datetime,
        size: int,
        hash: str,
        path: pathlib.Path,
    ) -> 'FileDescription':
        return cls(
            modified=modified,
            content=ContentDescription(size=size, hash=hash),
            path=pathlib.Path(path),
        )

    @classmethod
    def parse(cls, string: str, relative_to: pathlib.Path) -> 'FileDescription':
        modified, size, hash_, path = string.strip().split(',')
        return cls.create(
            modified=dateutil.parser.parse(modified),
            size=int(size),
            hash=hash_,
            path=relative_to / path,
        )

    def __str__(self) -> str:
        return ','.join((
            self.modified.isoformat(' '),
            str(self.content.size),
            self.content.hash,
            str(self.path),
        ))


MaybeFileDescription = Union[MissingFile, FileDescription]


def walk_files(directory: pathlib.Path) -> Iterator[pathlib.Path]:
    for root, dirs, files in os.walk(str(directory)):
        rootpath = pathlib.Path(root)
        for filename in files:
            yield rootpath / filename


def hash_file(filepath: pathlib.Path, hash=hashlib.md5) -> str:
    with filepath.open(mode='rb') as f:
        return hash(f.read(HASH_PREFIX_SIZE)).hexdigest()


def describe(filepath: pathlib.Path) -> MaybeFileDescription:
    try:
        stat = filepath.stat()
    except FileNotFoundError:
        return MissingFile(filepath)
    else:
        return FileDescription.create(
            modified=datetime.datetime.fromtimestamp(stat.st_mtime),
            size=stat.st_size,
            hash=hash_file(filepath),
            path=filepath,
        )


def path_by_content(descriptions: Iterable[FileDescription]) -> Dict[ContentDescription, pathlib.Path]:
    return {x.content: x.path for x in descriptions}


def load_descriptions(references: Iterable[TextIO]) -> Dict[pathlib.Path, FileDescription]:
    descriptions = [
        FileDescription.parse(l, relative_to=pathlib.Path(f.name).parent)
        for f in references
        for l in f
    ]
    return {x.path: x for x in descriptions}


def describe_differences(
    expected: Dict[pathlib.Path, FileDescription],
    current: Dict[pathlib.Path, MaybeFileDescription],
) -> Tuple[
    List[CopiedFile],
    List[MovedFile],
    List[MissingFile],
    List[FileDescription],
]:
    missing = []  # type: List[pathlib.Path]
    actual = {}  # type: Dict[pathlib.Path, FileDescription]
    unexpected = {}  # type: Dict[pathlib.Path, FileDescription]

    for filepath, description in current.items():
        if isinstance(description, MissingFile):
            missing.append(filepath)
        else:
            actual[filepath] = description
            if filepath not in expected:
                unexpected[filepath] = description

    path_by_expected_content = path_by_content(expected.values())
    path_by_actual_content = path_by_content(
        x for x in current.values() if isinstance(x, FileDescription),
    )

    copied = []  # type: List[CopiedFile]
    moved = []  # type: List[MovedFile]
    deleted = []  # type: List[MissingFile]
    new_files = []  # type: List[FileDescription]

    for missing_path in missing:
        expected_content = expected[missing_path].content
        if expected_content in path_by_actual_content:
            moved.append(MovedFile(
                old=missing_path,
                new=path_by_actual_content[expected_content],
            ))
        else:
            deleted.append(MissingFile(missing_path))

    for filepath, description in unexpected.items():
        content = description.content
        if content in path_by_expected_content:
            expected_path = path_by_expected_content[content]
            if expected_path in actual:
                copied.append(CopiedFile(old=expected_path, new=filepath))
        else:
            new_files.append(description)

    return copied, moved, deleted, new_files


@click.group()
def cli():
    pass


@cli.command()
@click.argument('directory', type=click.Path(exists=True))
def record(directory: str) -> None:
    """Record the current state of the directory"""
    for filepath in walk_files(pathlib.Path(directory)):
        print(describe(filepath))


@cli.command()
@click.argument('directory', type=click.Path(exists=True))
@click.argument('references', required=True, nargs=-1, type=click.File('rt'))
def audit(directory: str, references: Iterable[TextIO]) -> None:
    """Audit records in the given files"""

    expected = load_descriptions(references)
    actual = {p: describe(p) for p in expected.keys()}

    for filepath in walk_files(pathlib.Path(directory)):
        if filepath not in actual:
            actual[filepath] = describe(filepath)

    copied, moved, deleted, new_files = describe_differences(expected, actual)

    if copied:
        print("# Copied files:")
        for copy in sorted(copied):
            print(f"{copy.new} (from {copy.old})")

    if moved:
        print("# Moved files:")
        for move in sorted(moved):
            print(f"{move.new} (from {move.old})")

    if deleted:
        print("# Deleted files:")
        for deleted_file in sorted(deleted):
            print(deleted_file.path)

    if new_files:
        print("# New files:")
        for new_file in sorted(new_files):
            print(new_file)


if __name__ == '__main__':
    cli()
