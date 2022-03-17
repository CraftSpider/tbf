//! The TBF, short for 'tag-based filesystem', is a new way of storing files.
//!
//! Optimized for human recall and easy searching, tag-based storage reduces the need
//! for complex storage trees. Instead, every file has a unique machine ID, as well as
//! various tagged metadata, which can be used to find any set of files at any time.
//!
//! The overall storage system works like this:
//! - Files are added to the network, and automatically assigned various metadata tags
//! - The user is free to add new tags, which may be part of a tag 'group'
//! - Alternatively, the user can use a unique ID to access a file
//!
//! The system is defined as a trait, with various implementations able to use their own backing
//! implementations. This could be an existing standard filesystem, a SQL database, or just
//! in-memory maps.

#![warn(
    missing_docs,
    elided_lifetimes_in_paths,
    explicit_outlives_requirements,
    missing_abi,
    noop_method_call,
    pointer_structural_match,
    semicolon_in_expressions_from_macros,
    unused_import_braces,
    unused_lifetimes,
    clippy::cargo,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::ptr_as_ptr,
    clippy::cloned_instead_of_copied,
    clippy::unreadable_literal
)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "dfs")]
mod dfs;
#[cfg(feature = "imfs")]
mod imfs;
mod pattern;
mod file;
pub mod error;

#[cfg(feature = "dfs")]
pub use dfs::{DirectoryBackedFs, Error as DfsError};
#[cfg(feature = "imfs")]
pub use imfs::{Error as ImfsError, InMemoryFs};

pub use pattern::{TagPattern, TagPredicate};
pub use file::{FileId, Tag, Group};
pub use error::{Error, ErrorKind};

use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use alloc::vec::Vec;

/// A trait representing an implementation of a tag-based filesystem.
pub trait FileSystem {
    /// The error type to use with this filesystem.
    type Error: Error;

    // Add/Remove/Edit files

    /// Add a new file with the given data and tags
    fn add_file<I>(&self, data: &[u8], tags: I) -> Result<FileId, Self::Error>
    where
        I: IntoIterator<Item = Tag>;

    /// Edit an existing file, altering the data or tags
    fn edit_file<I>(
        &self,
        id: FileId,
        data: Option<&[u8]>,
        tags: Option<I>,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Tag>;

    /// Remove an existing file
    fn remove_file(&self, id: FileId) -> Result<(), Self::Error>;

    // Lookup files

    /// Search for files matching a given tag pattern
    fn search_tags<P>(&self, tags: P) -> Result<Vec<FileId>, Self::Error>
    where
        P: TagPattern;

    /// Get info about an existing file
    fn get_info(&self, id: FileId) -> Result<FileInfo, Self::Error>;
}

/// Combined info about a file
#[derive(Debug)]
pub struct FileInfo {
    id: FileId,
    tags: BTreeSet<Tag>,
    data: Box<[u8]>,
}

impl FileInfo {
    /// Get the ID of this file
    pub fn id(&self) -> FileId {
        self.id
    }

    /// Get the tags associated with this file
    pub fn tags(&self) -> &BTreeSet<Tag> {
        &self.tags
    }

    /// Get the raw data associated with this file
    pub fn data(&self) -> &[u8] {
        &*self.data
    }
}
