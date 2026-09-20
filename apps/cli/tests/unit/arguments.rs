use super::*;

#[test]
fn preserves_feature_arguments_and_normalizes_options() {
    let cases = [
        (
            vec!["todo", "list", "--date", "2026-09-20"],
            vec!["todo", "--date", "2026-09-20", "list"],
        ),
        (
            vec!["memo", "create", "hello", "world"],
            vec!["memo", "create", "hello world"],
        ),
        (
            vec!["memo", "update", "id", "--file", "note.md"],
            vec!["memo", "update", "id", "--file", "note.md"],
        ),
        (
            vec!["memo", "create", "--", "--help"],
            vec!["memo", "create", "--", "--help"],
        ),
        (
            vec!["moment", "upload-photo", "{}", "photo.heic"],
            vec!["moment", "upload-photo", "{}", "photo.heic"],
        ),
        (
            vec![
                "moment",
                "upload-photo",
                "--file",
                "metadata.json",
                "photo.heic",
            ],
            vec![
                "moment",
                "upload-photo",
                "--file",
                "metadata.json",
                "photo.heic",
            ],
        ),
        (
            vec!["moment", "upload-photo", "--stdin", "photo.heic"],
            vec!["moment", "upload-photo", "--stdin", "photo.heic"],
        ),
    ];
    for (input, expected) in cases {
        let parsed = parse(
            std::iter::once("vesper")
                .chain(input.clone())
                .map(OsString::from),
        )
        .unwrap_or_else(|error| panic!("{input:?}: {error}"));
        assert_eq!(parsed, expected, "{input:?}");
    }
}
