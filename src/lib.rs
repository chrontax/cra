//! Simple abstraction over archive formats.
//!
//! You can read and write archives in zip, 7z, and tar formats.

use infer::get;
use sevenz_rust::{nt_time::FileTime, Password, SevenZArchiveEntry, SevenZReader, SevenZWriter};
use std::{
    io::{self, Cursor, Read, Write},
    time::{SystemTime, UNIX_EPOCH},
};
use tar::{Archive as TarArchive, Builder as TarBuilder, Entry as TarEntry, Header};
use thiserror::Error;
use uzers::{get_current_gid, get_current_groupname, get_current_uid, get_current_username};
use zip::{read::ZipFile, write::SimpleFileOptions, ZipArchive, ZipWriter};

/// Enum representing supported archive formats
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArcFormat {
    Zip,
    Tar,
    Sevenz,
}

impl TryFrom<infer::Type> for ArcFormat {
    type Error = ArcError;

    fn try_from(value: infer::Type) -> Result<Self, Self::Error> {
        Ok(match value.extension() {
            "zip" => ArcFormat::Zip,
            "7z" => ArcFormat::Sevenz,
            "tar" => ArcFormat::Tar,
            _ => return Err(ArcError::UnrecognizedFormat),
        })
    }
}

/// Enum representing an archive entry
///
/// Can be a directory with a name or a file with a name and data.
#[derive(Debug, Clone, PartialEq)]
pub enum ArcEntry {
    File(String, Vec<u8>),
    Directory(String),
}

impl From<ZipFile<'_>> for ArcEntry {
    fn from(mut entry: ZipFile) -> Self {
        if entry.is_dir() {
            ArcEntry::Directory(entry.name().to_owned())
        } else {
            let mut data = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut data).unwrap();
            ArcEntry::File(entry.name().to_owned(), data)
        }
    }
}

impl From<TarEntry<'_, &[u8]>> for ArcEntry {
    fn from(mut entry: TarEntry<'_, &[u8]>) -> Self {
        let name = entry.path().unwrap().to_str().unwrap().to_owned();
        if entry.header().entry_type().is_dir() {
            ArcEntry::Directory(name)
        } else {
            let mut data = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut data).unwrap();
            ArcEntry::File(name, data)
        }
    }
}

/// Main error type for this library
#[derive(Error, Debug)]
#[error(transparent)]
pub enum ArcError {
    IoError(#[from] io::Error),
    ZipError(#[from] zip::result::ZipError),
    SevenzError(#[from] sevenz_rust::Error),
    #[error("Unrecognized archive format")]
    UnrecognizedFormat,
}

pub type ArcResult<T> = Result<T, ArcError>;

/// This struct allows you to easily read an archive
pub struct ArcReader {
    format: ArcFormat,
    entries: Vec<ArcEntry>,
    i: usize,
}

impl ArcReader {
    /// Takes the archive to read as a slice of bytes and reads it
    pub fn new(buf: &[u8]) -> ArcResult<Self> {
        let format = get(buf).unwrap().try_into()?;
        Ok(Self {
            format,
            entries: match format {
                ArcFormat::Zip => ArcReader::read_zip(buf),
                ArcFormat::Tar => ArcReader::read_tar(buf),
                ArcFormat::Sevenz => ArcReader::read_7z(buf),
            }?,
            i: 0,
        })
    }

    /// Returns the format of the archive
    pub fn format(&self) -> ArcFormat {
        self.format
    }

    /// Returns a reference to all archive entries
    pub fn entries(&self) -> &Vec<ArcEntry> {
        &self.entries
    }

    fn read_zip(buf: &[u8]) -> ArcResult<Vec<ArcEntry>> {
        let mut archive = ZipArchive::new(Cursor::new(buf)).unwrap();
        let len = archive.len();
        let mut entries = Vec::with_capacity(len);
        for i in 0..len {
            entries.push(archive.by_index(i)?.into());
        }
        Ok(entries)
    }

    fn read_tar(buf: &[u8]) -> ArcResult<Vec<ArcEntry>> {
        Ok(TarArchive::new(buf)
            .entries()?
            .map(|entry| entry.unwrap().into())
            .collect())
    }

    fn read_7z(buf: &[u8]) -> ArcResult<Vec<ArcEntry>> {
        let mut entries = Vec::new();
        SevenZReader::new(Cursor::new(buf), buf.len() as u64, Password::empty())?
            .for_each_entries(|entry, reader| {
                if entry.is_directory {
                    entries.push(ArcEntry::Directory(entry.name.clone()));
                } else {
                    let mut data = Vec::with_capacity(entry.size as usize);
                    reader.read_to_end(&mut data).unwrap();
                    entries.push(ArcEntry::File(entry.name.clone(), data));
                }
                Ok(true)
            })
            .unwrap();
        Ok(entries)
    }
}

impl Iterator for ArcReader {
    type Item = ArcEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i == self.entries.len() {
            None
        } else {
            self.i += 1;
            Some(self.entries[self.i - 1].clone())
        }
    }
}

/// Struct for creating archives
pub struct ArcWriter {
    pub format: ArcFormat,
    entries: Vec<ArcEntry>,
}

impl ArcWriter {
    /// Returns a new writer for the specified archive format
    pub fn new(format: ArcFormat) -> Self {
        Self {
            format,
            entries: Vec::new(),
        }
    }

    /// Adds an entry to the writer
    pub fn push(&mut self, entry: ArcEntry) {
        self.entries.push(entry)
    }

    /// Adds all entries from slice to the writer
    pub fn extend(&mut self, entries: &[ArcEntry]) {
        self.entries.extend_from_slice(entries)
    }

    /// Creates the finished archive
    pub fn archive(&self) -> ArcResult<Vec<u8>> {
        match self.format {
            ArcFormat::Zip => self.archive_zip(),
            ArcFormat::Tar => self.archive_tar(),
            ArcFormat::Sevenz => self.archive_7z(),
        }
    }

    fn archive_zip(&self) -> ArcResult<Vec<u8>> {
        let mut inner = Vec::new();
        {
            let mut writer = ZipWriter::new(Cursor::new(&mut inner));
            for entry in &self.entries {
                match entry {
                    ArcEntry::Directory(name) => {
                        writer.add_directory(name, SimpleFileOptions::default())?
                    }
                    ArcEntry::File(name, data) => {
                        writer.start_file(name.as_str(), SimpleFileOptions::default())?;
                        writer.write_all(data)?;
                    }
                }
            }
            writer.finish()?;
        }
        Ok(inner)
    }

    fn archive_tar(&self) -> ArcResult<Vec<u8>> {
        let mut inner = Vec::new();
        {
            let mut builder = TarBuilder::new(&mut inner);
            for entry in &self.entries {
                let mut header = Header::new_gnu();
                header.set_mode(0o766);
                header.set_mtime(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                );
                header.set_uid(get_current_uid() as u64);
                header
                    .set_username(get_current_username().unwrap().to_str().unwrap())
                    .unwrap();
                header.set_gid(get_current_gid() as u64);
                header
                    .set_groupname(get_current_groupname().unwrap().to_str().unwrap())
                    .unwrap();
                match entry {
                    ArcEntry::Directory(name) => {
                        header.set_entry_type(tar::EntryType::Directory);
                        builder.append_data(&mut header, name, &[][..])?;
                    }
                    ArcEntry::File(name, data) => {
                        header.set_entry_type(tar::EntryType::Regular);
                        header.set_size(data.len() as u64);
                        builder.append_data(&mut header, name, &data[..])?;
                    }
                }
            }
            builder.finish()?;
        }
        Ok(inner)
    }

    fn archive_7z(&self) -> ArcResult<Vec<u8>> {
        let mut inner = Vec::new();
        let mut archive = SevenZWriter::new(Cursor::new(&mut inner))?;
        for entry in &self.entries {
            let mut szentry = SevenZArchiveEntry::default();
            szentry.has_last_modified_date = true;
            szentry.last_modified_date = FileTime::now();
            match entry {
                ArcEntry::Directory(name) => {
                    szentry.is_directory = true;
                    szentry.name = name.clone();
                    archive.push_archive_entry::<&[u8]>(szentry, None)?;
                }
                ArcEntry::File(name, data) => {
                    szentry.name = name.clone();
                    archive.push_archive_entry(szentry, Some(&data[..]))?;
                }
            }
        }
        archive.finish()?;
        Ok(inner)
    }
}
