#!/usr/bin/env python3

from __future__ import annotations

import datetime
import hashlib
import os.path
import pathlib
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    Iterable,
    Iterator,
    Mapping,
    NamedTuple,
    Protocol,
    TextIO,
    TypeVar,
    Union,
)

import click
import dateutil.parser

if TYPE_CHECKING:
    from hashlib import _Hash  # noqa: F401


T = TypeVar('T')


class SupportsLessThan(Protocol):
    def __lt__(self, __other: Any) -> bool: ...


SupportsLessThanT = TypeVar("SupportsLessThanT", bound=SupportsLessThan)

HASH_PREFIX_SIZE: int = 1024 * 1024


class ContentDescription(NamedTuple):
    size: int
    hash: str


class MovedFile(NamedTuple):
    old: pathlib.Path
    new: pathlib.Path


class CopiedFile(NamedTuple):
    old: pathlib.Path
    new: pathlib.Path


class NewFile(NamedTuple):
    path: pathlib.Path


class ChangedFile(NamedTuple):
    path: pathlib.Path
    old_content: ContentDescription
    new_content: ContentDescription


class MissingFile(NamedTuple):
    path: pathlib.Path


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
    ) -> FileDescription:
        return cls(
            modified=modified,
            content=ContentDescription(size=size, hash=hash),
            path=pathlib.Path(path),
        )

    @classmethod
    def parse(cls, string: str, relative_to: pathlib.Path) -> FileDescription:
        modified, size, hash_, path = string.strip().split(',', maxsplit=3)
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


class _ChangeSummary(NamedTuple):
    changed: list[ChangedFile]
    copied: list[CopiedFile]
    moved: list[MovedFile]
    deleted: list[MissingFile]
    added: list[FileDescription]


class ChangeSummary(_ChangeSummary):
    def __bool__(self) -> bool:
        return any(x for x in self)

    def describe(self) -> str:
        def descriptions(items: list[SupportsLessThanT], title: str, template: str) -> str | None:
            if not items:
                return None

            items_description = "\n".join(
                template.format(x)
                for x in sorted(items)
            )
            return f"# {title}:\n{items_description}"

        return "\n".join(x for x in (
            descriptions(self.changed, "Changed files", "{0.path}"),
            descriptions(self.copied, "Copied files", "{0.new} (from {0.old})"),
            descriptions(self.moved, "Moved files", "{0.new} (from {0.old})"),
            descriptions(self.deleted, "Deleted files", "{0.path}"),
            descriptions(self.added, "Added files", "{0}"),
        ) if x)


MaybeFileDescription = Union[MissingFile, FileDescription]


def walk_files(directory: pathlib.Path) -> Iterator[pathlib.Path]:
    for root, dirs, files in os.walk(str(directory)):
        rootpath = pathlib.Path(root)
        for filename in files:
            yield rootpath / filename


def hash_file(filepath: pathlib.Path, hash: Callable[[bytes], _Hash] = hashlib.md5) -> str:
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


def path_by_content(descriptions: Iterable[FileDescription]) -> dict[ContentDescription, pathlib.Path]:
    return {x.content: x.path for x in descriptions}


def load_descriptions(references: Iterable[TextIO]) -> dict[pathlib.Path, FileDescription]:
    descriptions = [
        FileDescription.parse(
            line,
            relative_to=pathlib.Path(reference.name).parent,
        )
        for reference in references
        for line in reference
    ]
    return {x.path: x for x in descriptions}


def describe_differences(
    expected: Mapping[pathlib.Path, FileDescription],
    current: Mapping[pathlib.Path, MaybeFileDescription],
) -> ChangeSummary:
    missing: list[pathlib.Path] = []
    actual: dict[pathlib.Path, FileDescription] = {}
    unexpected: dict[pathlib.Path, FileDescription] = {}

    changed: list[ChangedFile] = []

    for filepath, description in current.items():
        if isinstance(description, MissingFile):
            missing.append(filepath)
        else:
            actual[filepath] = description
            expected_description = expected.get(filepath)
            if expected_description is None:
                unexpected[filepath] = description
            elif expected_description != description:
                changed.append(ChangedFile(
                    path=filepath,
                    old_content=expected_description.content,
                    new_content=description.content,
                ))

    path_by_expected_content = path_by_content(expected.values())
    path_by_actual_content = path_by_content(
        x for x in current.values() if isinstance(x, FileDescription)
    )

    copied: list[CopiedFile] = []
    moved: list[MovedFile] = []
    deleted: list[MissingFile] = []
    new_files: list[FileDescription] = []

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

    return ChangeSummary(changed, copied, moved, deleted, new_files)


@click.group()
def cli() -> None:
    pass


@cli.command()
@click.argument('directory', type=click.Path(exists=True))
def record(directory: str) -> None:
    """
    Record the current state of the directory.

    This captures the current state of the directory and prints it in a format
    which can later be used as the input for other commands.
    """
    for filepath in walk_files(pathlib.Path(directory)):
        print(describe(filepath))


@cli.command()
@click.argument('directory', type=click.Path(exists=True))
@click.argument('references', required=True, nargs=-1, type=click.File('rt'))
def audit(directory: str, references: Iterable[TextIO]) -> None:
    """
    Audit records in the given files.

    This captures the current state of the directory and the compares it to the
    state expected by the given references.
    """

    expected = load_descriptions(references)
    current = {p: describe(p) for p in expected.keys()}

    for filepath in walk_files(pathlib.Path(directory)):
        if filepath not in current:
            current[filepath] = describe(filepath)

    change_summary = describe_differences(expected, current)

    if change_summary:
        print(change_summary.describe())


@cli.command()
@click.argument('baseline', type=click.File('rt'))
@click.argument('target', type=click.File('rt'))
def compare(baseline: TextIO, target: TextIO) -> None:
    """Compare records in the given files."""

    baseline_descriptions = load_descriptions([baseline])
    target_descriptions: dict[pathlib.Path, MaybeFileDescription] = dict(
        load_descriptions([target]),
    )

    for filepath in baseline_descriptions.keys():
        if filepath not in target_descriptions:
            target_descriptions[filepath] = MissingFile(filepath)

    change_summary = describe_differences(
        baseline_descriptions,
        target_descriptions,
    )

    if change_summary:
        print(change_summary.describe())


@cli.command('find-duplicates')
@click.argument('references', required=True, nargs=-1, type=click.File('rt'))
def find_duplicates(references: list[TextIO]) -> None:
    """Search for duplicates within the given files."""

    all_by_content: dict[ContentDescription, list[pathlib.Path]] = {}

    descriptions = load_descriptions(references)
    for path, description in descriptions.items():
        all_by_content.setdefault(description.content, []).append(path)

    duplicates = {x: y for x, y, in all_by_content.items() if len(y) > 1}

    if not duplicates:
        print("No duplicates")
        return

    for content, paths in duplicates.items():
        print(f"Duplicate content {content.hash} (size {content.size})")
        paths.sort()
        for path in paths:
            print(f" - {path}")


if __name__ == '__main__':
    cli()
