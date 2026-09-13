use super::{BuildError, build};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

#[tokio::test]
async fn compiles_content() {
    let repository = temporary_directory("compile");
    let content = repository.join("content/posts");
    fs::create_dir_all(&content).unwrap();
    fs::write(content.join("hello.md"), "# Hello").unwrap();
    fs::write(content.join("photo.png"), [1, 2, 3]).unwrap();

    let output = build(&repository).await.unwrap();
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

#[tokio::test]
async fn detects_collision() {
    let repository = temporary_directory("collision");
    let content = repository.join("content");
    fs::create_dir_all(&content).unwrap();
    fs::write(content.join("page.md"), "# Markdown").unwrap();
    fs::write(content.join("page.html"), "already HTML").unwrap();

    let error = build(&repository).await.unwrap_err();

    assert!(matches!(error, BuildError::OutputCollision(..)));
    fs::remove_dir_all(repository).unwrap();
}

#[tokio::test]
async fn rejects_bad_mermaid() {
    let repository = temporary_directory("invalid-mermaid");
    let content = repository.join("content");
    fs::create_dir_all(&content).unwrap();
    fs::write(content.join("page.md"), "```mermaid\nnot-a-diagram\n```").unwrap();

    let error = build(&repository).await.unwrap_err();

    assert!(matches!(error, BuildError::Markdown { .. }));
    fs::remove_dir_all(repository).unwrap();
}

#[tokio::test]
async fn rejects_bad_content_embed() {
    let repository = temporary_directory("invalid-content-embed");
    let content = repository.join("content");
    fs::create_dir_all(&content).unwrap();
    fs::write(
        content.join("page.md"),
        "```embed:github\nrepo: missing-owner\n```",
    )
    .unwrap();

    let error = build(&repository).await.unwrap_err();

    assert!(matches!(error, BuildError::Markdown { .. }));
    fs::remove_dir_all(repository).unwrap();
}

#[tokio::test]
#[ignore = "requires GitHub CLI access"]
async fn stores_rendered_output() {
    let repository = temporary_directory("rich-markdown");
    let content = repository.join("content");
    fs::create_dir_all(&content).unwrap();
    fs::write(
        content.join("page.md"),
        "```rust\nfn main() {}\n```\n\n```mermaid\nflowchart LR\nA --> B\n```\n\n```embed:github\nrepo: canmi21/seam\nalign: left\n```",
    )
    .unwrap();

    let output = build(&repository).await.unwrap();
    let html = fs::read_to_string(output.directory().join("page.html")).unwrap();
    let index: serde_json::Value =
        serde_json::from_slice(&fs::read(output.directory().join("content.json")).unwrap())
            .unwrap();

    assert!(html.contains("class=\"highlighted-code\""));
    assert!(html.contains("class=\"mermaid-diagram\"><svg"));
    assert!(html.contains("content-embed-github content-embed-left"));
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

#[tokio::test]
async fn copies_local_media_beside_compiled_articles() {
    let repository = temporary_directory("media");
    let content = repository.join("content");
    fs::create_dir_all(content.join("posts")).unwrap();
    fs::create_dir_all(content.join("media")).unwrap();
    fs::write(content.join("media/片段 one.mp3"), b"audio").unwrap();
    fs::write(content.join("media/cover.jpg"), b"poster").unwrap();
    fs::write(content.join("posts/article.md"), "# Media\n\n```embed:media\ntype: audio\nsrc: ../media/片段 one.mp3\n```\n\n```embed:media\ntype: video\nsrc: https://example.com/demo.mp4\nposter: ../media/cover.jpg\n```").unwrap();
    let output = build(&repository).await.unwrap();
    assert_eq!(output.report().copied_files, 2);
    let html = fs::read_to_string(output.directory().join("posts/article.html")).unwrap();
    assert!(html.contains("src=\"../media/%E7%89%87%E6%AE%B5%20one.mp3\""));
    assert!(html.contains("src=\"https://example.com/demo.mp4\""));
    assert_eq!(
        fs::read(output.directory().join("media/片段 one.mp3")).unwrap(),
        b"audio"
    );
    assert_eq!(
        fs::read(output.directory().join("media/cover.jpg")).unwrap(),
        b"poster"
    );
    let index: serde_json::Value =
        serde_json::from_slice(&fs::read(output.directory().join("content.json")).unwrap())
            .unwrap();
    assert_eq!(index["documents"][0]["html"], html);
    fs::remove_dir_all(repository).unwrap();
}

#[tokio::test]
async fn rejects_missing_or_unpublished_media() {
    let repository = temporary_directory("invalid-media");
    let content = repository.join("content");
    fs::create_dir_all(&content).unwrap();
    fs::write(repository.join("private.mp3"), b"private").unwrap();
    fs::write(content.join("other.md"), "# Other").unwrap();
    fs::write(content.join("audio.mp3"), b"audio").unwrap();
    let absolute = content
        .join("audio.mp3")
        .to_string_lossy()
        .replace('/', "%2F");
    for src in [
        "missing.mp3",
        "../private.mp3",
        "%2E%2E/private.mp3",
        &absolute,
        "other.md",
        ".",
    ] {
        for source in [
            format!("```embed:media\ntype: audio\nsrc: {src}\n```"),
            format!(
                "Text[^note]\n\n[^note]:\n    ```embed:media\n    type: audio\n    src: {src}\n    ```\n"
            ),
        ] {
            fs::write(content.join("article.md"), source).unwrap();
            assert!(
                matches!(
                    build(&repository).await.unwrap_err(),
                    BuildError::Media { .. }
                ),
                "{src}"
            );
        }
    }
    fs::remove_dir_all(repository).unwrap();
}
