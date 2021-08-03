use super::{Tag, Group};

use core::borrow::Borrow;
use alloc::boxed::Box;
use alloc::vec::Vec;

mod sealed {
    use super::*;

    pub trait Sealed {}

    impl Sealed for Tag {}
    impl Sealed for [Tag] {}
    impl<const N: usize> Sealed for [Tag; N] {}
    impl Sealed for TagPredicate {}
}

/// Any type that can be used to match a file's tags on
pub trait TagPattern: sealed::Sealed {
    /// Match this item against an iterator of tags
    fn match_tags<T, I>(&self, tags: I) -> bool
    where
        T: Borrow<Tag>,
        I: IntoIterator<Item = T>;
}

impl TagPattern for Tag {
    fn match_tags<T, I>(&self, tags: I) -> bool
    where
        T: Borrow<Tag>,
        I: IntoIterator<Item = T>,
    {
        tags.into_iter().any(|tag| tag.borrow() == self)
    }
}

impl TagPattern for [Tag] {
    fn match_tags<T, I>(&self, tags: I) -> bool
    where
        T: Borrow<Tag>,
        I: IntoIterator<Item = T>,
    {
        let tags = tags.into_iter().collect::<Vec<_>>();
        self.iter().all(|tag| tags.iter().any(|t| tag == t.borrow()))
    }
}

impl<const N: usize> TagPattern for [Tag; N] {
    fn match_tags<T, I>(&self, tags: I) -> bool
    where
        T: Borrow<Tag>,
        I: IntoIterator<Item = T>,
    {
        <[Tag]>::match_tags(self, tags)
    }
}

/// Complex support for matching binary expressions against tags
pub enum TagPredicate {
    /// And predicates together
    And(Vec<TagPredicate>),
    /// Or predicates together
    Or(Vec<TagPredicate>),
    /// Inverse a predicate
    Not(Box<TagPredicate>),

    /// Match just the group of a tag
    Group(Group),
    /// Match just the name of a tag
    Name(String),
    /// Match a tag exactly
    Tag(Tag),
}

impl From<Tag> for TagPredicate {
    fn from(tag: Tag) -> TagPredicate {
        TagPredicate::Tag(tag)
    }
}

impl TagPredicate {
    /// Create an and predicate from an iterator of predicate items
    pub fn and<T, I>(preds: I) -> TagPredicate
    where
        T: Into<TagPredicate>,
        I: IntoIterator<Item = T>,
    {
        TagPredicate::And(preds.into_iter().map(T::into).collect())
    }

    /// Create an or predicate from an iterator of predicate items
    pub fn or<T, I>(preds: I) -> TagPredicate
    where
        T: Into<TagPredicate>,
        I: IntoIterator<Item = T>,
    {
        TagPredicate::Or(preds.into_iter().map(T::into).collect())
    }

    /// Create a not predicate from some other predicate item
    pub fn not<T>(pred: T) -> TagPredicate
    where
        T: Into<TagPredicate>,
    {
        TagPredicate::Not(Box::new(pred.into()))
    }

    /// Create a predicate for a group
    pub fn group(group: Group) -> TagPredicate {
        TagPredicate::Group(group)
    }

    /// Create a predicate for a name
    pub fn name(name: &str) -> TagPredicate {
        TagPredicate::Name(name.to_string())
    }

    /// Create a predicate to match a tag exactly
    pub fn tag(tag: Tag) -> TagPredicate {
        TagPredicate::Tag(tag)
    }
}

impl TagPattern for TagPredicate {
    fn match_tags<T, I>(&self, tags: I) -> bool
    where
        T: Borrow<Tag>,
        I: IntoIterator<Item = T>,
    {
        use TagPredicate::*;
        let mut iter = tags.into_iter();
        match self {
            And(preds) => {
                let tags = iter.collect::<Vec<_>>();
                preds.iter().all(|pred| pred.match_tags(tags.iter().map(|item| item.borrow())))
            },
            Or(preds) => {
                let tags = iter.collect::<Vec<_>>();
                preds.iter().any(|pred| pred.match_tags(tags.iter().map(|item| item.borrow())))
            },
            Not(pred) => !pred.match_tags(iter),

            Group(group) => iter.any(|tag| &tag.borrow().group == group),
            Name(name) => iter.any(|tag| &tag.borrow().name == name),
            Tag(tag) => tag.match_tags(iter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag() {
        let tag_a = Tag::name("a");

        assert!(tag_a.match_tags(&[Tag::name("a"), Tag::name("b"),]));
        assert!(!tag_a.match_tags(&[Tag::name("b"), Tag::name("c"),]));
    }

    #[test]
    fn test_tag_slice() {
        let tag_slice = &[Tag::name("a"), Tag::name("b")];

        assert!(tag_slice.match_tags(&[
            Tag::name("b"),
            Tag::name("c"),
            Tag::name("d"),
            Tag::name("a"),
        ]));

        assert!(!tag_slice.match_tags(&[Tag::name("c"), Tag::name("ab"), Tag::name("d"),]))
    }

    #[test]
    fn test_pred_and() {
        let pred = TagPredicate::and([Tag::name("a"), Tag::name("b")]);

        assert!(pred.match_tags(&[
            Tag::name("c"),
            Tag::name("b"),
            Tag::name("f"),
            Tag::name("a"),
        ]));
        assert!(!pred.match_tags(&[Tag::name("c"), Tag::name("f"), Tag::name("a"),]));
    }

    #[test]
    fn test_pred_or() {
        let pred = TagPredicate::or([Tag::name("a"), Tag::name("b")]);

        assert!(pred.match_tags(&[Tag::name("a"),]));
        assert!(pred.match_tags(&[Tag::name("b"), Tag::name("c"),]));
        assert!(!pred.match_tags(&[Tag::name("c"), Tag::name("d"),]));
    }

    #[test]
    fn test_pred_not() {
        let pred = TagPredicate::not(Tag::name("a"));

        assert!(pred.match_tags(&[Tag::name("b"), Tag::name("c")]));
        assert!(!pred.match_tags(&[Tag::name("a"), Tag::name("b")]));
    }

    #[test]
    fn test_pred_group() {
        let pred = TagPredicate::group(Group::Default);

        assert!(pred.match_tags(&[
            Tag::name("a"),
            Tag::new(Group::custom("group"), "a")
        ]));
        assert!(!pred.match_tags(&[
            Tag::new(Group::custom("group"), "a"),
            Tag::new(Group::custom("group"), "b")
        ]));
    }

    #[test]
    fn test_pred_name() {
        let pred = TagPredicate::name("a");

        assert!(pred.match_tags(&[
            Tag::new(Group::custom("group"), "a"),
            Tag::new(Group::custom("group"), "b"),
        ]));
        assert!(pred.match_tags(&[
            Tag::name("a"),
            Tag::name("b"),
        ]));
        assert!(!pred.match_tags(&[
            Tag::new(Group::custom("group"), "b"),
            Tag::name("b"),
        ]));
    }

    #[test]
    fn test_pred_tag() {
        let pred = TagPredicate::Tag(Tag::name("a"));

        assert!(pred.match_tags(&[Tag::name("c"), Tag::name("a"),]));
        assert!(!pred.match_tags(&[Tag::name("c"), Tag::name("f"),]));
    }
}
