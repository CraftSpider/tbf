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

#[cfg(feature = "dfs")]
pub use dfs::{DirectoryBackedFs, Error as DfsError};
#[cfg(feature = "imfs")]
pub use imfs::{Error as ImfsError, InMemoryFs};

pub use pattern::{TagPattern, TagPredicate};

use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::convert::TryFrom;

/// A trait representing an implementation of a tag-based filesystem.
pub trait FileSystem {
    /// The error type to use with this filesystem.
    type Error;

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

/// Represents the ID of a file. Most numbers simply represent a unique file, however,
/// the values 0-255 are reserved for special usage.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FileId(u64);

impl FileId {
    /// Check if this ID represents a special file
    pub fn is_special(self) -> bool {
        self.0 <= 255
    }

    /// Check if this ID represents a standard file
    pub fn is_file(self) -> bool {
        self.0 > 255
    }

    /// Create a `FileId` from a `u64`, without checking that the value is in the reserved range
    pub fn from_u64_unchecked(id: u64) -> Self {
        FileId(id)
    }

    /// Create a `u64` from a `FileId`, without checking that the value is in the reserved range
    pub fn into_u64_unchecked(self) -> u64 {
        self.0
    }
}

impl TryFrom<u64> for FileId {
    type Error = ();

    fn try_from(val: u64) -> Result<Self, Self::Error> {
        if val <= 255 {
            Err(())
        } else {
            Ok(FileId(val))
        }
    }
}

impl TryFrom<FileId> for u64 {
    type Error = ();

    fn try_from(id: FileId) -> Result<Self, Self::Error> {
        if id.is_special() {
            Err(())
        } else {
            Ok(id.0)
        }
    }
}

/// The group associated with a tag. Many tags will be part of the 'default'
/// group, but there can be any number of custom groups.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Group {
    /// The default group
    Default,
    /// A group with a custom name
    Custom(String),
}

impl Group {
    /// Get the custom group associated with a given string
    pub fn custom(group: &str) -> Group {
        Group::Custom(group.to_string())
    }
}

impl Default for Group {
    fn default() -> Self {
        Group::Default
    }
}

/// A file tag, with a name and optionally a tag group
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tag {
    group: Group,
    name: String,
}

impl Tag {
    /// Create a new tag with both a group and tag name
    pub fn new(group: Group, name: &str) -> Tag {
        Tag {
            group,
            name: name.to_string(),
        }
    }

    /// Create a tag with a name in the default group
    pub fn named(name: &str) -> Tag {
        Tag {
            group: Group::Default,
            name: name.to_string(),
        }
    }

    /// Get the group for this tag
    pub fn group(&self) -> &Group {
        &self.group
    }

    /// Get the name of this tag
    pub fn name(&self) -> &str {
        &self.name
    }
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
