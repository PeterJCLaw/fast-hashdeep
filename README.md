# `fast-hashdeep`

A tool for _quickly_ getting an idea for whether directories of large files have
the same contents. This is similar to the much more rigorous `hashdeep`, though
is _much_ faster for large files.

The primary speedup vs related tools comes from not actually checking the full
content of each file, so it can only give a general idea about changes. This is
an acceptable trade-off where file integrity is not an issue and where file
diversity is large.

The original use-case was for coping with directories of video files which might
have been moved or renamed, but which were unlikely to actually _change_.

## Testing

Very minimal tests exist via the `./runtests` script. This operates on some
dummy data files and directories within `tests/` and can be configured to use
either the Python or Rust implementations.

Python:
```
./runtests ./fast-hashdeep.py
```

Rust:
```
./runtests cargo run --release --
```
