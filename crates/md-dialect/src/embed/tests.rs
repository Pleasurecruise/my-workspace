use super::*;

#[test]
fn renders_repository_data() {
    let mut data = Data::default();
    data.repositories.insert(
        "canmi21/seam".to_owned(),
        quotes::github::RepositorySnapshot {
            full_name: "canmi21/seam".to_owned(),
            description: "A typed seam".to_owned(),
            owner_avatar_url: "https://avatars.example/canmi21".to_owned(),
            language: "Rust".to_owned(),
            stars: 21,
            forks: 3,
            open_issues: 2,
            default_branch: "main".to_owned(),
            updated_at: "2026-09-01T00:00:00Z".to_owned(),
            url: "https://github.com/canmi21/seam".to_owned(),
        },
    );
    let html = render(GITHUB, "repo: canmi21/seam\nalign: left", &data)
        .expect("valid repository embed")
        .expect("registered embed");
    assert!(html.contains("content-embed-left"));
    assert!(html.contains(">21</span>"));
    assert!(html.contains(">2</span>"));
}

#[test]
fn renders_stock_data_as_a_smooth_month_chart() {
    let mut data = Data::default();
    data.stocks.insert(
        "AAPL".to_owned(),
        quotes::stocks::StockSeries {
            symbol: "AAPL".to_owned(),
            name: "Apple Inc.".to_owned(),
            currency: "USD".to_owned(),
            exchange: "NMS".to_owned(),
            price: 231.4,
            change: 2.1,
            change_percent: 0.92,
            points: vec![
                quotes::stocks::StockPoint {
                    timestamp: 1,
                    close: 220.0,
                },
                quotes::stocks::StockPoint {
                    timestamp: 2,
                    close: 226.0,
                },
                quotes::stocks::StockPoint {
                    timestamp: 3,
                    close: 231.4,
                },
            ],
        },
    );
    let html = render(STOCK, "code: AAPL\nalign: right", &data)
        .expect("valid stock embed")
        .expect("registered embed");
    assert!(html.contains("content-embed-right content-embed-up"));
    assert!(html.contains("class=\"content-stock-area\""));
    assert!(html.contains(" C"));
    assert!(html.contains("<circle class=\"content-stock-end\""));
}

#[test]
fn renders_link_metadata_as_text_and_keeps_ordinary_links_unhandled() {
    let mut data = Data::default();
    data.links.insert(
        "https://example.com".to_owned(),
        quotes::opengraph::Metadata {
            url: "https://example.com/?a=1&b=2".to_owned(),
            title: "<script>alert(1)</script>".to_owned(),
            description: "A & B".to_owned(),
            site_name: "Example".to_owned(),
            image: None,
        },
    );
    let html = render(LINK, "url: https://example.com\nalign: left", &data)
        .unwrap()
        .unwrap();
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(html.contains("A &amp; B"));
    assert!(html.contains("href=\"https://example.com/?a=1&amp;b=2\""));
    assert!(!html.contains("<img"));
    assert!(
        render("text", "https://example.com", &data)
            .unwrap()
            .is_none()
    );
    assert!(render(LINK, "url: https://example.com\nimage: injected", &data).is_err());
}

#[test]
fn renders_audio_and_video_without_provider_data() {
    let data = Data::default();
    let audio = render(
        MEDIA,
        "type: audio\nsrc: ./media/片段 one.mp3\ntitle: A & B\ncaption: <script>literal</script>",
        &data,
    )
    .unwrap()
    .unwrap();
    assert!(audio.contains("<audio controls preload=\"none\""));
    assert!(audio.contains("src=\"./media/%E7%89%87%E6%AE%B5%20one.mp3\""));
    assert!(audio.contains("aria-label=\"A &amp; B\""));
    assert!(audio.contains("<figcaption>&lt;script&gt;literal&lt;/script&gt;</figcaption>"));
    assert!(!audio.contains("autoplay"));
    let video = render(MEDIA, "type: video\nsrc: https://example.com/watch.mp4?a=1&b=2#t=5\nposter: ../media/cover.jpg\nalign: left", &data).unwrap().unwrap();
    assert!(video.contains("content-embed-left"));
    assert!(
        video
            .contains("<video controls preload=\"none\" playsinline poster=\"../media/cover.jpg\"")
    );
    assert!(video.contains("watch.mp4?a=1&amp;b=2#t=5"));
}

#[test]
fn rejects_invalid_media_fields_and_sources() {
    let data = Data::default();
    for fields in [
        "type: image\nsrc: photo.png",
        "type: audio",
        "src: media.mp4",
        "type: audio\nsrc: a.mp3\nposter: cover.jpg",
        "type: video\nsrc: a.mp4\nautoplay: true",
        "type: video\nsrc: a.mp4\nsrc: b.mp4",
        "type: video\nsrc: a.mp4\nalign: center",
    ] {
        assert!(render(MEDIA, fields, &data).is_err(), "{fields}");
    }
    for src in [
        "javascript:alert(1)",
        "data:audio/mpeg;base64,abcd",
        "file:///tmp/a.mp3",
        "/Users/name/a.mp3",
        "//example.com/a.mp3",
        "C:\\media\\a.mp3",
        "https://user:secret@example.com/a.mp3",
        "~/Music/a.mp3",
        "a.mp3#fragment",
        "https://",
        "https://example.com/\u{0000}a.mp3",
    ] {
        assert!(
            render(MEDIA, &format!("type: audio\nsrc: {src}"), &data).is_err(),
            "{src}"
        );
        assert!(
            render(
                MEDIA,
                &format!("type: video\nsrc: a.mp4\nposter: {src}"),
                &data
            )
            .is_err(),
            "poster: {src}"
        );
    }
}

#[test]
fn collects_only_local_media_assets() {
    let source = "```embed:media\ntype: audio\nsrc: ./audio.mp3\n```\n\n```EMBED:MEDIA\ntype: video\nsrc: https://example.com/video.mp4\nposter: ./cover.jpg\n```\n\n```embed:media\ntype: audio\nsrc: ./audio.mp3\n```\n\n```text\ntype: audio\nsrc: ignored.mp3\n```";
    assert_eq!(
        collect_media_paths(source).unwrap(),
        vec!["./audio.mp3", "./cover.jpg"]
    );
}

#[test]
fn previews_video_frames_without_overriding_posters_or_start_times() {
    let data = Data::default();
    let preview = render(MEDIA, "type: video\nsrc: ./video.mp4", &data)
        .unwrap()
        .unwrap();
    assert!(preview.contains("src=\"./video.mp4#t=0.001\""));
    assert!(preview.contains("controls preload=\"metadata\" playsinline"));
    assert!(preview.contains("href=\"./video.mp4\""));
    assert!(!preview.contains("autoplay"));
    let poster = render(
        MEDIA,
        "type: video\nsrc: ./video.mp4\nposter: ./cover.jpg",
        &data,
    )
    .unwrap()
    .unwrap();
    assert!(poster.contains("src=\"./video.mp4\""));
    assert!(poster.contains("poster=\"./cover.jpg\""));
    assert!(poster.contains("preload=\"none\""));
    let timed = render(
        MEDIA,
        "type: video\nsrc: https://example.com/demo.mp4#t=5,10",
        &data,
    )
    .unwrap()
    .unwrap();
    assert!(timed.contains("src=\"https://example.com/demo.mp4#t=5,10\""));
    assert!(!timed.contains("0.001"));
}

#[test]
fn github_media_file_pages_resolve_to_bytes_without_rewriting_other_hosts() {
    let data = Default::default();
    for (source, expected) in [
        (
            "https://github.com/Pleasurecruise/pleasure1234/blob/main/public/cat.mp3",
            "https://raw.githubusercontent.com/Pleasurecruise/pleasure1234/main/public/cat.mp3",
        ),
        (
            "https://github.com/a/b/blob/feature/audio/my%20clip.mp3?raw=true#t=5",
            "https://raw.githubusercontent.com/a/b/feature/audio/my%20clip.mp3#t=5",
        ),
        (
            "https://github.com/a/b/releases/download/v1/song.mp3",
            "https://github.com/a/b/releases/download/v1/song.mp3",
        ),
        (
            "https://github.com.example.com/a/b/blob/main/song.mp3",
            "https://github.com.example.com/a/b/blob/main/song.mp3",
        ),
        (
            "https://github.com/a/b/blob/main",
            "https://github.com/a/b/blob/main",
        ),
    ] {
        let html = render(MEDIA, &format!("type: audio\nsrc: {source}"), &data)
            .unwrap()
            .unwrap();
        assert!(html.contains(&format!("src=\"{expected}\"")), "{html}");
    }
}
