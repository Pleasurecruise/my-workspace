use super::collect_files;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

#[test]
fn collects_nested_files() {
    let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "vesper-publish-test-{}-{sequence}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("posts")).unwrap();
    fs::write(root.join("content.json"), "{}").unwrap();
    fs::write(root.join("posts/hello.html"), "hello").unwrap();

    let mut files = collect_files(&root, &root)
        .unwrap()
        .into_iter()
        .map(|file| file.1)
        .collect::<Vec<String>>();
    files.sort();
    assert_eq!(files, ["content.json", "posts/hello.html"]);
    fs::remove_dir_all(root).unwrap();
}
