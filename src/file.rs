use core::convert::TryFrom;
use alloc::string::{String, ToString};

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
