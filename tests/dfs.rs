use std::collections::BTreeSet;
use tempdir::TempDir;
use tbf::{DirectoryBackedFs, FileSystem, Group, Tag};

#[test]
fn rw_file() {
    let test_dir = TempDir::new("test_dfs")
        .unwrap();

    let dfs = DirectoryBackedFs::new(test_dir.path())
        .unwrap();

    let id = dfs.add_file(&[0, 1, 2, 3], [Tag::named("a"), Tag::new(Group::custom("g"), "b")])
        .unwrap();

    let info = dfs.get_info(id)
        .unwrap();

    assert_eq!(info.data(), &[0, 1, 2, 3]);
    assert_eq!(info.tags(), &BTreeSet::from([
        Tag::named("a"),
        Tag::new(Group::custom("g"), "b"),
    ]));
}
