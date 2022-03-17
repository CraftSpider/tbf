//! Existing file-system backed implementation of a TBF

use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{PoisonError, RwLock};
use std::{fs, io};

use super::{FileId, FileInfo, FileSystem};
use crate::{Group, Tag, TagPattern};
use crate::error::Kind;

/// Error for a directory-backed filesystem
#[derive(Debug)]
pub enum Error {
    /// A file wasn't found
    FileNotFound(FileId),
    /// A thread panic poisoned the state
    Poisoned,
    /// An I/O error occured
    IoError(io::Error),
}

impl<T> From<PoisonError<T>> for Error {
    fn from(_: PoisonError<T>) -> Error {
        Error::Poisoned
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IoError(err)
    }
}

impl crate::error::Error for Error {
    fn file_not_found(id: FileId) -> Self {
        Self::FileNotFound(id)
    }

    fn generic_kind(&self) -> Kind<'_> {
        match self {
            Self::FileNotFound(id) => Kind::FileNotFound(*id),
            Self::IoError(e) => Kind::Source(e),
            Self::Poisoned => Kind::State,
        }
    }
}

struct SavedState {
    cur_id: u64,
}

impl SavedState {
    fn from_path(path: &Path) -> Result<SavedState, Error> {
        match File::open(path) {
            Ok(mut file) => {
                let mut cur_id = [0; 8];
                file.read_exact(&mut cur_id)?;

                Ok(SavedState {
                    cur_id: u64::from_le_bytes(cur_id),
                })
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(SavedState { cur_id: 256 }),
            Err(err) => Err(err.into()),
        }
    }

    fn save(&self, path: &Path) -> Result<(), Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        file.write_all(&self.cur_id.to_le_bytes()).unwrap();
        Ok(())
    }
}

struct TagIter {
    back: io::Bytes<BufReader<File>>,
}

impl TagIter {
    fn new(back: io::Bytes<BufReader<File>>) -> TagIter {
        TagIter { back }
    }

    fn read_u32(&mut self) -> Option<u32> {
        let a = self.back.next()?.unwrap();
        let b = self.back.next()?.unwrap();
        let c = self.back.next()?.unwrap();
        let d = self.back.next()?.unwrap();

        Some(u32::from_le_bytes([a, b, c, d]))
    }

    fn read_string(&mut self) -> Option<String> {
        let len = self.read_u32()?;
        let mut bytes = Vec::new();
        for _ in 0..len {
            bytes.push(self.back.next()?.unwrap());
        }
        String::from_utf8(bytes).ok()
    }
}

impl Iterator for TagIter {
    type Item = Tag;

    fn next(&mut self) -> Option<Self::Item> {
        let has_group = self.back.next()?.unwrap();

        let group = if has_group == 1 {
            Group::Custom(self.read_string()?)
        } else {
            Group::Default
        };

        let name = self.read_string()?;

        Some(Tag::new(group, &name))
    }
}

/// A directory-backed implementation of a tag-based filesystem. Given a directory on a standard
/// filesystem, will persist all data there.
pub struct DirectoryBackedFs {
    dir: PathBuf,
    state: RwLock<SavedState>,
}

impl DirectoryBackedFs {
    /// Create or load a directory-backed filesystem, in the provided directory.
    pub fn new(dir: PathBuf) -> Result<DirectoryBackedFs, Error> {
        if !dir.exists() {
            fs::create_dir_all(dir.clone())?;
        } else if !dir.is_dir() {
            return Err(Error::IoError(io::Error::new(
                io::ErrorKind::Other,
                "Provided path exists and is not a directory",
            )));
        }

        let state = RwLock::new(SavedState::from_path(&dir.join("tbf.dat"))?);

        Ok(DirectoryBackedFs { dir, state })
    }

    fn assert_dir(&self) -> Result<(), Error> {
        if self.dir.is_dir() {
            Ok(())
        } else {
            Err(Error::IoError(io::Error::new(
                io::ErrorKind::Other,
                "Provided path exists and is not a directory",
            )))
        }
    }

    fn file_name(&self, id: FileId) -> PathBuf {
        self.dir.join(format!("{:016X}", id.into_u64_unchecked()))
    }

    fn write_tags<I>(&self, id: FileId, tags: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = Tag>,
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.file_name(id).with_extension(".tag"))?;

        tags.into_iter()
            .try_for_each::<_, Result<(), Error>>(|tag| {
                if let Group::Custom(_) = tag.group() {
                    file.write_all(&[1])?;
                } else {
                    file.write_all(&[0])?;
                }

                match tag.group() {
                    Group::Custom(group) => {
                        file.write_all(&[1])?;
                        file.write_all(&(group.len() as u32).to_le_bytes())?;
                        file.write_all(group.as_bytes())?;
                    }
                    Group::Default => file.write_all(&[0])?,
                }

                file.write_all(&(tag.name().len() as u32).to_le_bytes())?;
                file.write_all(tag.name().as_bytes())?;

                Ok(())
            })?;

        Ok(())
    }

    fn read_tags(&self, id: FileId) -> Result<impl Iterator<Item = Tag>, Error> {
        let back =
            io::BufReader::new(File::open(self.file_name(id).with_extension(".tag"))?).bytes();
        Ok(TagIter::new(back))
    }
}

impl FileSystem for DirectoryBackedFs {
    type Error = Error;

    fn add_file<I>(&self, data: &[u8], tags: I) -> Result<FileId, Self::Error>
    where
        I: IntoIterator<Item = Tag>,
    {
        self.assert_dir()?;
        let cur_id = FileId::from_u64_unchecked(self.state.read()?.cur_id);
        fs::write(self.file_name(cur_id).with_extension(".dat"), data)?;
        self.write_tags(cur_id, tags)?;
        self.state.write()?.cur_id += 1;
        self.state.read()?.save(&self.dir.join("tbf.dat"))?;
        Ok(cur_id)
    }

    fn edit_file<I>(
        &self,
        id: FileId,
        data: Option<&[u8]>,
        tags: Option<I>,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Tag>,
    {
        self.assert_dir()?;
        if let Some(data) = data {
            fs::write(self.file_name(id).with_extension(".dat"), data)?;
        }
        if let Some(tags) = tags {
            self.write_tags(id, tags)?;
        }
        Ok(())
    }

    fn remove_file(&self, id: FileId) -> Result<(), Self::Error> {
        self.assert_dir()?;
        let _ = fs::remove_file(self.file_name(id).with_extension(".dat"));
        let _ = fs::remove_file(self.file_name(id).with_extension(".tag"));
        Ok(())
    }

    fn search_tags<P>(&self, tags: P) -> Result<Vec<FileId>, Self::Error>
    where
        P: TagPattern,
    {
        self.assert_dir()?;
        let mut out = Vec::new();
        for item in fs::read_dir(&self.dir)? {
            let item = item?;
            let file_name = match item.file_name().to_str() {
                Some(name) => name.to_owned(),
                None => continue,
            };
            let (id, ext) = match file_name.split_once('.') {
                Some(val) => val,
                None => continue,
            };

            if ext != "tag" {
                continue;
            }
            let id = match id.parse() {
                Ok(val) => FileId::from_u64_unchecked(val),
                Err(_) => continue,
            };

            if tags.match_tags(self.read_tags(id)?) {
                out.push(id)
            }
        }
        Ok(out)
    }

    fn get_info(&self, id: FileId) -> Result<FileInfo, Self::Error> {
        self.assert_dir()?;
        let data = fs::read(self.file_name(id).with_extension(".dat"))?.into_boxed_slice();
        let tags = self.read_tags(id)?.collect();
        Ok(FileInfo { id, data, tags })
    }
}
