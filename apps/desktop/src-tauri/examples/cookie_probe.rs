#[cfg(target_os = "macos")]
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let window = tauri::WebviewWindowBuilder::new(&app, "cookie-probe", tauri::WebviewUrl::External("about:blank".parse()?))
                        .incognito(true).visible(false).build()?;
                    window.set_cookie(tauri::webview::Cookie::build(("probe", "synthetic-value")).domain(".mihoyo.com").path("/").secure(true).build())?;
                    let all = window.cookies()?;
                    let filtered = window.cookies_for_url("https://webstatic.mihoyo.com/".parse()?)?;
                    let installed = all.iter().any(|cookie| cookie.name() == "probe" && cookie.value() == "synthetic-value" && cookie.domain().is_some_and(|domain| domain.trim_start_matches('.') == "mihoyo.com")
                        && cookie.path() == Some("/") && cookie.secure() == Some(true));
                    println!("native WebView created; parent cookie installed: {installed}; exact-domain URL getter count: {}", filtered.len());
                    if !installed || !filtered.is_empty() {
                        return Err("Native cookie behavior differs from the regression scenario".into());
                    }
                    window.close()?;
                    Ok(())
                })();
                match result {
                    Ok(()) => app.exit(0),
                    Err(error) => { eprintln!("probe failed: {error}"); app.exit(1); }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!("tests/fixtures/cookie-probe/tauri.conf.json"))
        .expect("run isolated cookie probe");
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("This probe reproduces the macOS WebView parent-domain cookie filter.");
}
