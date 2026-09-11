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
