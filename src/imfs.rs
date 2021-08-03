//! In-memory implementation of a TBF

#[cfg(not(feature = "std"))]
use spin::{RwLock, RwLockReadGuard as ReadGuard, RwLockWriteGuard as WriteGuard};
#[cfg(feature = "std")]
use std::sync::{
    PoisonError, RwLock, RwLockReadGuard as ReadGuard, RwLockWriteGuard as WriteGuard,
};

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use super::{FileId, FileInfo, FileSystem, Tag, TagPattern};

type FileData = Vec<Box<[u8]>>;
type TagData = BTreeMap<FileId, BTreeSet<Tag>>;

/// Error for an in-memory filesystem
#[derive(Debug)]
pub enum Error {
    /// The requested file did not exist
    FileNotFound,
    /// The filesystem was poisoned by a thread panic
    Poisoned,
}

#[cfg(feature = "std")]
impl<T> From<PoisonError<T>> for Error {
    fn from(_: PoisonError<T>) -> Error {
        Error::Poisoned
    }
}

/// An in-memory implementation of a tag-based filesystem. This implementation
/// will store all data in program memory, only persisting it for the duration of the
/// program runtime.
///
/// This is most useful for tests / mocking of a filesystem, and probably not what you want
/// for long term usage.
pub struct InMemoryFs {
    files: RwLock<FileData>,
    tags: RwLock<TagData>,
}

impl InMemoryFs {
    /// Create a new instance of an in-memory filesystem
    pub fn new() -> InMemoryFs {
        InMemoryFs {
            files: RwLock::new(Vec::new()),
            tags: RwLock::new(BTreeMap::new()),
        }
    }

    fn read_files(&self) -> Result<ReadGuard<'_, FileData>, Error> {
        #[cfg(feature = "std")]
        let out = self.files.read()?;
        #[cfg(not(feature = "std"))]
        let out = self.files.read();
        Ok(out)
    }

    fn write_files(&self) -> Result<WriteGuard<'_, FileData>, Error> {
        #[cfg(feature = "std")]
        let out = self.files.write()?;
        #[cfg(not(feature = "std"))]
        let out = self.files.write();
        Ok(out)
    }

    fn read_tags(&self) -> Result<ReadGuard<'_, TagData>, Error> {
        #[cfg(feature = "std")]
        let out = self.tags.read()?;
        #[cfg(not(feature = "std"))]
        let out = self.tags.read();
        Ok(out)
    }

    fn write_tags(&self) -> Result<WriteGuard<'_, TagData>, Error> {
        #[cfg(feature = "std")]
        let out = self.tags.write()?;
        #[cfg(not(feature = "std"))]
        let out = self.tags.write();
        Ok(out)
    }

    fn assert_file_exists(&self, id: FileId) -> Result<(), Error> {
        self.read_tags()?
            .get(&id)
            .map(|_| ())
            .ok_or(Error::FileNotFound)
    }
}

impl Default for InMemoryFs {
    fn default() -> Self {
        InMemoryFs::new()
    }
}

impl FileSystem for InMemoryFs {
    type Error = Error;

    fn add_file<I>(&self, data: &[u8], tags: I) -> Result<FileId, Self::Error>
    where
        I: IntoIterator<Item = Tag>,
    {
        let new_id = {
            let mut files = self.write_files()?;
            files.push(data.to_owned().into_boxed_slice());

            FileId(files.len() as u64 + 255)
        };

        let mut tags_map = self.write_tags()?;
        tags_map.insert(new_id, tags.into_iter().collect());

        Ok(new_id)
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
        self.assert_file_exists(id)?;

        if let Some(data) = data {
            let mut files = self.write_files()?;
            files[(id.0 - 255) as usize] = data.to_owned().into_boxed_slice();
        }
        if let Some(tags) = tags {
            let mut tags_map = self.write_tags()?;
            tags_map.insert(id, tags.into_iter().collect());
        }

        Ok(())
    }

    fn remove_file(&self, id: FileId) -> Result<(), Self::Error> {
        self.assert_file_exists(id)?;

        let mut files = self.write_files()?;
        files[(id.0 - 255) as usize] = Box::new([]) as Box<[u8]>;
        let mut tags_map = self.write_tags()?;
        tags_map.remove(&id);
        Ok(())
    }

    fn search_tags<P>(&self, tags: P) -> Result<Vec<FileId>, Self::Error>
    where
        P: TagPattern,
    {
        let mut out = Vec::new();
        for (id, file_tags) in self.read_tags()?.iter() {
            if tags.match_tags(file_tags) {
                out.push(*id)
            }
        }
        Ok(out)
    }

    fn get_info(&self, id: FileId) -> Result<FileInfo, Self::Error> {
        self.assert_file_exists(id)?;

        Ok(FileInfo {
            id,
            data: self.read_files()?[(id.0 - 255) as usize].clone(),
            tags: self.read_tags()?.get(&id).unwrap().clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_add_file() {
        let ifs = InMemoryFs::new();

        let id = ifs.add_file(&[0, 1, 2], []).unwrap();

        assert_eq!(id, FileId(256));
    }

    #[test]
    pub fn test_search_files() {
        let ifs = InMemoryFs::new();

        let first = ifs.add_file(&[0, 1, 2], [Tag::name("a"), Tag::name("b")])
            .unwrap();
        let second = ifs.add_file(&[0, 1, 2], [Tag::name("a")])
            .unwrap();
        let third = ifs.add_file(&[0, 1, 2], [Tag::name("b")])
            .unwrap();
        let fourth = ifs.add_file(&[0, 1, 2], [Tag::name("c"), Tag::name("a")])
            .unwrap();

        let items = ifs.search_tags(Tag::name("a"))
            .unwrap();

        assert!(items.contains(&first) && items.contains(&second) && items.contains(&fourth));
        assert!(!items.contains(&third));

        let items = ifs.search_tags(Tag::name("b"))
            .unwrap();

        assert!(items.contains(&first) && items.contains(&third));
        assert!(!items.contains(&second) && !items.contains(&fourth));
    }
}
