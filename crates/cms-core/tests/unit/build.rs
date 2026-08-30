use super::{BuildError, build};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

#[test]
fn compiles_content() {
    let repository = temporary_directory("compile");
    let content = repository.join("content/posts");
    fs::create_dir_all(&content).unwrap();
    fs::write(content.join("hello.md"), "# Hello").unwrap();
    fs::write(content.join("photo.png"), [1, 2, 3]).unwrap();

    let output = build(&repository).unwrap();
    let report = output.report();

    assert_eq!(report.markdown_files, 1);
    assert_eq!(report.copied_files, 1);
    assert_eq!(
        fs::read_to_string(output.directory().join("posts/hello.html")).unwrap(),
        "<h1>Hello</h1>\n"
    );
    assert_eq!(
        fs::read(output.directory().join("posts/photo.png")).unwrap(),
        [1, 2, 3]
    );
    let index: serde_json::Value =
        serde_json::from_slice(&fs::read(output.directory().join("content.json")).unwrap())
            .unwrap();
    assert_eq!(index["version"], 1);
    assert_eq!(index["documents"][0]["path"], "posts/hello.html");
    assert_eq!(index["documents"][0]["html"], "<h1>Hello</h1>\n");
    let output_directory = output.directory().to_owned();
    drop(output);
    assert!(!output_directory.exists());
    fs::remove_dir_all(repository).unwrap();
}

#[test]
fn detects_collision() {
    let repository = temporary_directory("collision");
    let content = repository.join("content");
    fs::create_dir_all(&content).unwrap();
    fs::write(content.join("page.md"), "# Markdown").unwrap();
    fs::write(content.join("page.html"), "already HTML").unwrap();

    let error = build(&repository).unwrap_err();

    assert!(matches!(error, BuildError::OutputCollision(..)));
    fs::remove_dir_all(repository).unwrap();
}

#[test]
fn rejects_bad_mermaid() {
    let repository = temporary_directory("invalid-mermaid");
    let content = repository.join("content");
    fs::create_dir_all(&content).unwrap();
    fs::write(content.join("page.md"), "```mermaid\nnot-a-diagram\n```").unwrap();

    let error = build(&repository).unwrap_err();

    assert!(matches!(error, BuildError::Markdown { .. }));
    fs::remove_dir_all(repository).unwrap();
}

#[test]
fn stores_rendered_output() {
    let repository = temporary_directory("rich-markdown");
    let content = repository.join("content");
    fs::create_dir_all(&content).unwrap();
    fs::write(
        content.join("page.md"),
        "```rust\nfn main() {}\n```\n\n```mermaid\nflowchart LR\nA --> B\n```",
    )
    .unwrap();

    let output = build(&repository).unwrap();
    let html = fs::read_to_string(output.directory().join("page.html")).unwrap();
    let index: serde_json::Value =
        serde_json::from_slice(&fs::read(output.directory().join("content.json")).unwrap())
            .unwrap();

    assert!(html.contains("class=\"highlighted-code\""));
    assert!(html.contains("class=\"mermaid-diagram\"><svg"));
    assert_eq!(index["documents"][0]["html"], html);
    drop(output);
    fs::remove_dir_all(repository).unwrap();
}

fn temporary_directory(name: &str) -> PathBuf {
    let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "my-workspace-{name}-{}-{sequence}",
        std::process::id()
    ));
    if path.exists() {
        fs::remove_dir_all(&path).unwrap();
    }
    fs::create_dir_all(&path).unwrap();
    path
}
