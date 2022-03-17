//! Common error trait and kind for all implementations of the main trait

use core::marker::PhantomData;

use crate::FileId;

/// The generic kind of a TBF error. This abstracts the most common error possibilities for
/// implementations. Some implementations may never produce errors with a specific kind, so if
/// you know the implementation, it's better to work with its error directly.
#[non_exhaustive]
pub enum ErrorKind<'a> {
    /// Error was for a file ID that doesn't exist
    FileNotFound(FileId),
    /// Error was caused by another error being returned in the implementation. Only present
    /// with the std feature for now
    #[cfg(feature = "std")]
    Source(&'a (dyn std::error::Error + Send + Sync)),
    /// Error was due to an invalid state in the filesystem
    State,
    /// Error was caused by something else
    Other,
    /// Variant to ensure `'a` is always used, shouldn't be matched on directly
    __Phantom(PhantomData<&'a ()>),
}

/// A common trait for all tag-based filesystem errors
pub trait Error: Sized {
    /// Create an instance of this error for a file that wasn't found
    fn file_not_found(id: FileId) -> Self;

    /// Get the generic kind of this error
    fn generic_kind(&self) -> ErrorKind<'_>;
}
