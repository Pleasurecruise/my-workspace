use super::*;

#[test]
fn only_remote_telemetry_widgets_enable_ugos() {
    for widget in [
        Widget::Cpu,
        Widget::Memory,
        Widget::Storage,
        Widget::Network,
    ] {
        let mut layout = Layout {
            widgets: vec![Placement {
                id: "nas".to_owned(),
                widget,
            }],
        };
        assert!(layout.has_ugos());
        layout.widgets.clear();
        assert!(!layout.has_ugos());
    }
    let layout = Layout {
        widgets: [
            Widget::LocalCpu,
            Widget::LocalMemory,
            Widget::LocalStorage,
            Widget::LocalNetwork,
            Widget::Steam,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, widget)| Placement {
            id: index.to_string(),
            widget,
        })
        .collect(),
    };
    assert!(!layout.has_ugos());
}
