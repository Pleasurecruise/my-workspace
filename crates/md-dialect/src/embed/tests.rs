use super::*;

#[test]
fn renders_repository_data_with_lucide_metadata_icons() {
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
    assert!(html.contains("<circle cx=\"12\" cy=\"12\" r=\"10\"/>"));
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
fn renders_storyboard_without_a_frame() {
    let html = render(
        STORYBOARD,
        "title: Publish\nstep: Write | Markdown\nstep: Ship | Website",
        &Data::default(),
    )
    .expect("valid storyboard")
    .expect("registered embed");
    assert!(html.contains("class=\"arrow-shadow\""));
    assert!(html.contains("class=\"arrow\""));
    assert!(!style::CSS.contains(".svg-canvas-storyboard{border:"));
}

#[test]
fn renders_canvases_on_the_consumer_theme() {
    assert!(style::CSS.contains(".svg-canvas{margin:.75rem auto"));
    assert!(style::CSS.contains(".svg-canvas-architecture>svg{background:transparent"));
    assert!(style::CSS.contains(".svg-canvas-storyboard>svg{color:"));
    assert!(style::CSS.contains("background:transparent"));
}
