use crate::CommandResponse;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Geometry {
    top_inset: f64,
    notch_width: f64,
}

#[tauri::command]
pub(crate) fn island_available() -> bool {
    cfg!(target_os = "macos")
}

pub(crate) fn sync(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let handle = app.clone();
        if let Err(error) = app.run_on_main_thread(move || {
            if let Err(error) = macos::sync(&handle) {
                tracing::warn!(%error, "could not update the macOS Dynamic Island");
            }
        }) {
            tracing::warn!(%error, "could not schedule the macOS Dynamic Island");
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

#[tauri::command]
pub(crate) async fn set_island_expanded(
    expanded: bool,
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
) -> CommandResponse<Geometry> {
    if window.label() != "island" {
        return CommandResponse::Failed {
            message: "This command belongs to the Dynamic Island".to_owned(),
        };
    }
    #[cfg(target_os = "macos")]
    {
        let (reply, response) = tokio::sync::oneshot::channel();
        let handle = app.clone();
        if let Err(error) = app.run_on_main_thread(move || {
            let _ = reply.send(macos::resize(&handle, expanded));
        }) {
            return CommandResponse::Failed {
                message: error.to_string(),
            };
        }
        match response.await {
            Ok(Ok(data)) => CommandResponse::Ready { data },
            Ok(Err(message)) => CommandResponse::Failed { message },
            Err(_) => CommandResponse::Failed {
                message: "Dynamic Island window closed".to_owned(),
            },
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (expanded, app);
        CommandResponse::Failed {
            message: "Dynamic Island is available on macOS".to_owned(),
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::Geometry;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSScreen, NSStatusWindowLevel, NSWindow, NSWindowCollectionBehavior, NSWorkspace,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

    pub(super) fn sync(app: &tauri::AppHandle) -> Result<(), String> {
        let visible = !app.state::<crate::configuration::AppLockState>().locked()
            && crate::widgets::island_widget(app)?.is_some();
        if !visible {
            if let Some(window) = app.get_webview_window("island") {
                window.destroy().map_err(|error| error.to_string())?;
            }
            return Ok(());
        }
        if let Some(window) = app.get_webview_window("island") {
            window
                .emit("layout-changed", ())
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        let window = WebviewWindowBuilder::new(app, "island", WebviewUrl::App("index.html".into()))
            .title("Vesper Dynamic Island")
            .inner_size(240.0, 32.0)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .resizable(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .visible_on_all_workspaces(true)
            .focused(false)
            .visible(false)
            .build()
            .map_err(|error| error.to_string())?;
        let events = window.clone();
        window.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Focused(false)) {
                let _ = events.emit("island-collapse", ());
            }
        });
        Ok(())
    }

    pub(super) fn resize(app: &tauri::AppHandle, expanded: bool) -> Result<Geometry, String> {
        if app.state::<crate::configuration::AppLockState>().locked() {
            return Err("Vesper is locked".to_owned());
        }
        let mtm = MainThreadMarker::new().ok_or("Dynamic Island requires the main thread")?;
        let window = app
            .get_webview_window("island")
            .ok_or("Dynamic Island window is unavailable")?;
        let screens = NSScreen::screens(mtm);
        let screen = screens
            .firstObject()
            .ok_or("No macOS display is available")?;
        let frame = screen.frame();
        let top_inset = screen.safeAreaInsets().top;
        let left = screen.auxiliaryTopLeftArea();
        let right = screen.auxiliaryTopRightArea();
        let notch_width = if top_inset > 0.0 {
            (right.origin.x - left.origin.x - left.size.width).max(0.0)
        } else {
            0.0
        };
        let width = if expanded {
            560.0_f64.min(frame.size.width - 32.0)
        } else {
            212.0_f64.max(notch_width + 104.0)
        };
        let height = if expanded {
            (top_inset + 300.0).min(frame.size.height - 48.0)
        } else {
            top_inset.max(32.0)
        };
        let pointer = window.ns_window().map_err(|error| error.to_string())?;
        // Tauri owns this NSWindow; only borrow it on the AppKit main thread.
        let native = unsafe { &*pointer.cast::<NSWindow>() };
        native.setLevel(NSStatusWindowLevel);
        native.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
        native.setHidesOnDeactivate(false);
        native.setCanHide(false);
        native.setOpaque(false);
        native.setHasShadow(false);
        native.setFrame_display_animate(
            NSRect::new(
                NSPoint::new(
                    frame.origin.x + (frame.size.width - width) / 2.0,
                    frame.origin.y + frame.size.height - height,
                ),
                NSSize::new(width, height),
            ),
            true,
            native.isVisible()
                && !NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion(),
        );
        // Tauri's show() makes the macOS window key. Resizing after focus loss
        // must not take focus back from the main window or another application.
        if !native.isVisible() {
            native.orderFrontRegardless();
        }
        Ok(Geometry {
            top_inset,
            notch_width,
        })
    }
}
