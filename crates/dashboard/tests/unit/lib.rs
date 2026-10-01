use super::*;

#[test]
fn planner_habits() {
    let mut layout = Layout {
        widgets: vec![Placement {
            id: "planner".into(),
            widget: Widget::Planner {
                habits: vec![
                    Habit {
                        id: "read".into(),
                        name: "Read".into(),
                    },
                    Habit {
                        id: "walk".into(),
                        name: "Walk".into(),
                    },
                ],
            },
        }],
        island_widget_id: Some("planner".into()),
    };
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(database::FILE_NAME);
    write(&path, &layout).unwrap();
    assert_eq!(
        serde_json::to_value(read(&path).unwrap()).unwrap(),
        serde_json::to_value(&layout).unwrap()
    );
    let Widget::Planner { habits } = &mut layout.widgets[0].widget else {
        panic!("Expected Planner")
    };
    habits[1].name = "READ".into();
    assert!(layout.validate().is_err());
    for name in ["", "  Read", "Read\nmore"] {
        let Widget::Planner { habits } = &mut layout.widgets[0].widget else {
            panic!("Expected Planner")
        };
        habits[1].name = name.into();
        assert!(layout.validate().is_err());
    }
}

#[test]
fn spending_is_a_persisted_singleton_that_can_be_pinned() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(database::FILE_NAME);
    let mut layout = Layout {
        widgets: vec![Placement {
            id: "spending".into(),
            widget: Widget::Spending,
        }],
        island_widget_id: Some("spending".into()),
    };
    write(&path, &layout).unwrap();
    let restored = read(&path).unwrap();
    assert!(matches!(restored.widgets[0].widget, Widget::Spending));
    assert_eq!(restored.island_widget_id.as_deref(), Some("spending"));
    layout.widgets.push(Placement {
        id: "spending-other".into(),
        widget: Widget::Spending,
    });
    assert!(layout.validate().is_err());
    assert!(matches!(
        read(&path).unwrap().widgets[0].widget,
        Widget::Spending
    ));
}

#[test]
fn every_provider_follows_saved_widget_presence() {
    for (provider, widget) in [
        (Provider::Codex, Widget::Codex),
        (Provider::OpenCode, Widget::OpenCode),
        (Provider::Claude, Widget::Claude),
        (Provider::Codex, Widget::CodexClaude),
        (Provider::Claude, Widget::CodexClaude),
        (Provider::Grok, Widget::Grok),
        (Provider::Copilot, Widget::Copilot),
        (Provider::DeepSeek, Widget::DeepSeek),
        (Provider::CherryIn, Widget::CherryIn),
        (Provider::TokenFlux, Widget::TokenFlux),
        (Provider::DimAgent, Widget::DimAgent),
        (Provider::Github, Widget::Github),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(database::FILE_NAME);
        let mut layout = Layout {
            widgets: vec![Placement {
                id: "provider".to_owned(),
                widget,
            }],
            island_widget_id: None,
        };
        write(&path, &layout).unwrap();
        assert!(read(&path).unwrap().has_provider(provider));
        layout.widgets.clear();
        write(&path, &layout).unwrap();
        assert!(!read(&path).unwrap().has_provider(provider));
    }
}

#[test]
fn replaces_layout_and_preserves_it_when_validation_fails() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(database::FILE_NAME);
    let mut layout = Layout::default();
    layout.widgets.reverse();
    write(&path, &layout).unwrap();
    let saved = serde_json::to_value(read(&path).unwrap()).unwrap();
    assert_eq!(saved, serde_json::to_value(&layout).unwrap());
    layout.island_widget_id = Some("absent".to_owned());
    assert!(write(&path, &layout).is_err());
    assert_eq!(serde_json::to_value(read(&path).unwrap()).unwrap(), saved);
    write(
        &path,
        &Layout {
            widgets: vec![],
            island_widget_id: None,
        },
    )
    .unwrap();
    assert!(read(&path).unwrap().widgets.is_empty());
}

#[test]
fn database_failure_rolls_back_the_complete_layout() {
    use diesel::connection::SimpleConnection;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(database::FILE_NAME);
    write(&path, &Layout::default()).unwrap();
    let saved = serde_json::to_value(read(&path).unwrap()).unwrap();
    let mut connection = database::open(&path).unwrap();
    connection.batch_execute("CREATE TRIGGER reject_layout BEFORE INSERT ON dashboard_layout BEGIN SELECT RAISE(ABORT, 'test write failure'); END;").unwrap();
    assert!(
        write(
            &path,
            &Layout {
                widgets: vec![],
                island_widget_id: None
            }
        )
        .is_err()
    );
    assert_eq!(serde_json::to_value(read(&path).unwrap()).unwrap(), saved);
}

#[test]
fn duplicate_order() {
    use diesel::connection::SimpleConnection;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(database::FILE_NAME);
    let mut connection = database::open(&path).unwrap();
    connection
        .batch_execute(
            r#"
        INSERT INTO dashboard_widgets VALUES
          ('planner-a', 0, '{"kind":"planner","habits":[{"id":"read","name":"Read"}]}'),
          ('planner-b', 1, '{"kind":"planner","habits":[{"id":"walk","name":"Walk"}]}');
        INSERT INTO dashboard_layout VALUES (1, 'planner-a');
    "#,
        )
        .unwrap();
    let mut layout = read(&path).unwrap();
    assert!(matches!(layout.widgets[1].widget, Widget::Invalid { .. }));
    layout.widgets.reverse();
    write(&path, &layout).unwrap();
    assert_eq!(
        serde_json::to_value(read(&path).unwrap()).unwrap(),
        serde_json::to_value(&layout).unwrap(),
    );
}

#[test]
fn isolates_invalid_widgets() {
    for configuration in [
        "{",
        r#"{"kind":"calendar"}"#,
        r#"{"kind":"spending","extra":true}"#,
        r#"{"kind":"stock","symbol":""}"#,
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(database::FILE_NAME);
        let initial = Layout {
            widgets: vec![
                Placement {
                    id: "planner".into(),
                    widget: Widget::Planner { habits: vec![] },
                },
                Placement {
                    id: "spending".into(),
                    widget: Widget::Spending,
                },
            ],
            island_widget_id: Some("planner".into()),
        };
        write(&path, &initial).unwrap();
        let mut connection = database::open(&path).unwrap();
        diesel::update(dashboard_widgets::table.find("planner"))
            .set(dashboard_widgets::configuration.eq(configuration))
            .execute(&mut connection)
            .unwrap();
        let mut restored = read(&path).unwrap();
        assert_eq!(restored.widgets.len(), initial.widgets.len());
        assert_eq!(restored.island_widget_id.as_deref(), Some("planner"));
        let broken = restored.widgets.iter().find(|p| p.id == "planner").unwrap();
        assert!(
            matches!(&broken.widget, Widget::Invalid { configuration: raw, error } if raw == configuration && !error.is_empty()),
            "{configuration}: {:?}",
            broken.widget
        );
        assert!(
            restored
                .widgets
                .iter()
                .any(|p| matches!(p.widget, Widget::Spending))
        );
        restored.widgets.reverse();
        write(&path, &restored).unwrap();
        let value: String = dashboard_widgets::table
            .find("planner")
            .select(dashboard_widgets::configuration)
            .first(&mut connection)
            .unwrap();
        assert_eq!(value, configuration);
        assert_eq!(
            serde_json::to_value(read(&path).unwrap()).unwrap(),
            serde_json::to_value(&restored).unwrap()
        );
        restored.widgets.retain(|p| p.id != "planner");
        restored.island_widget_id = None;
        write(&path, &restored).unwrap();
        assert!(
            read(&path)
                .unwrap()
                .widgets
                .iter()
                .all(|p| !matches!(p.widget, Widget::Invalid { .. }))
        );
    }
}

#[test]
fn island_selection_must_reference_a_saved_widget() {
    let mut layout = Layout::default();
    assert!(layout.island_widget_id.is_none());
    let planner = layout
        .widgets
        .iter()
        .find(|placement| matches!(placement.widget, Widget::Planner { .. }))
        .unwrap();
    layout.island_widget_id = Some(planner.id.clone());
    layout.validate().unwrap();
    layout.island_widget_id = Some("absent".to_owned());
    assert!(layout.validate().is_err());
    layout.island_widget_id = None;
    layout.validate().unwrap();
}

#[test]
fn rejects_kind() {
    let json = br#"{"widgets":[{"id":"bad","widget":{"kind":"unsupported"}}]}"#;
    assert!(decode(json).is_err());
}

#[test]
fn rejects_duplicates() {
    let json = br#"{"widgets":[{"id":"cpu-1","widget":{"kind":"cpu"}},{"id":"cpu-2","widget":{"kind":"cpu"}}]}"#;
    assert!(decode(json).is_err());
}

#[test]
fn supports_weather() {
    let json = br#"{"widgets":[{"id":"weather-shanghai","widget":{"kind":"weather","location":"shanghai"}},{"id":"weather-ningbo","widget":{"kind":"weather","location":"ningbo"}}]}"#;

    assert!(decode(json).is_ok());
}

#[test]
fn supports_custom_location() {
    let json = br#"{"widgets":[{"id":"weather-custom","widget":{"kind":"weather","location":"Hangzhou, China"}}]}"#;

    assert!(decode(json).is_ok());
}

#[test]
fn rejects_noncanonical_location() {
    let json = br#"{"widgets":[{"id":"weather-custom","widget":{"kind":"weather","location":" Hangzhou "}}]}"#;

    assert!(decode(json).is_err());
}

#[test]
fn supports_known_service_status() {
    let json = br#"{"widgets":[{"id":"service-status-codex","widget":{"kind":"serviceStatus","serviceId":"codex"}}]}"#;

    assert!(decode(json).is_ok());
}

#[test]
fn supports_one_exchange_widget() {
    let json = br#"{"widgets":[{"id":"exchange","widget":{"kind":"exchange"}}]}"#;

    assert!(decode(json).is_ok());

    let duplicates = br#"{"widgets":[{"id":"exchange-1","widget":{"kind":"exchange"}},{"id":"exchange-2","widget":{"kind":"exchange"}}]}"#;
    assert!(decode(duplicates).is_err());
}

#[test]
fn supports_current_device_widgets_as_singletons() {
    let json = br#"{"widgets":[{"id":"local-cpu","widget":{"kind":"localCpu"}},{"id":"local-memory","widget":{"kind":"localMemory"}},{"id":"local-storage","widget":{"kind":"localStorage"}},{"id":"local-network","widget":{"kind":"localNetwork"}}]}"#;
    assert!(decode(json).is_ok());

    let duplicate = br#"{"widgets":[{"id":"local-cpu-1","widget":{"kind":"localCpu"}},{"id":"local-cpu-2","widget":{"kind":"localCpu"}}]}"#;
    assert!(decode(duplicate).is_err());
}

#[test]
fn default_layout_is_valid() {
    Layout::default().validate().unwrap();
}

#[test]
fn rejects_unknown_service_status() {
    let json = br#"{"widgets":[{"id":"service-status-other","widget":{"kind":"serviceStatus","serviceId":"other"}}]}"#;

    assert!(decode(json).is_err());
}

#[test]
fn rejects_extra_field() {
    let json = br#"{"revision":1,"widgets":[]}"#;

    assert!(decode(json).is_err());
}
