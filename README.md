# Cra

Simple library for extracting/archiving in multiple formats fully in memory

## Features

* effortlessly read archives and iterate over their entries
* support for 7z, zip and tar
* fully in memory
* create archives in any supported format

## Usage

``` sh
cargo add cra
```

## Examples

Read and iterate over archive:

``` rust
use cra::{ArcReader, ArcEntry};

let mut archive = ArcReader::new(&archive_bytes).unwrap();

for entry in archive {
    match entry {
        ArcEntry::File(name, data) => { /* do something */ }
        ArcEntry::Directory(name) => { /* do something else */ }
    }
}
```

Create a zip archive with a directory and a file:

``` rust
use cra::{ArcWriter, ArcEntry, ArcFormat};

let mut writer = ArcWriter::new(Format::Zip);

writer.push(ArcEntry::Directory(String::from("some_dir")));
writer.push(ArcEntry::File(String::from("some_file"), data));

let finished_archive = writer.archive().unwrap(); // Vec<u8>
```
