// Offline native transport check: the real captcha HTML runs against a synthetic SDK.
#[cfg(target_os = "macos")]
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let result = (|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    let fake_sdk = r#"window.initGeetest = (options, ready) => ready({onReady(fn){setTimeout(fn,0)},verify(){},onError(){},onSuccess(fn){setTimeout(fn,10)},getValidate(){return {geetest_challenge:'mock-challenge',geetest_validate:'mock-validate',geetest_seccode:'mock-validate|jordan'}}});"#;
                    let sdk = format!("data:text/javascript,{}", percent_encoding::utf8_percent_encode(fake_sdk, percent_encoding::NON_ALPHANUMERIC));
                    let html = include_str!("../src/gaming/captcha.html")
                        .replace("__VESPER_CAPTCHA__", r#"{"gt":"mock-gt","challenge":"mock-challenge"}"#)
                        .replace("https://static.geetest.com/static/js/gt.0.5.2.js", &sdk);
                    let url: reqwest::Url = format!("data:text/html,{}", percent_encoding::utf8_percent_encode(&html, percent_encoding::NON_ALPHANUMERIC)).parse()?;
                    let handle = app.clone();
                    tauri::WebviewWindowBuilder::new(&app, "captcha-probe", tauri::WebviewUrl::External(url.clone()))
                        .title("Offline captcha probe").visible(false).incognito(true)
                        .on_navigation(move |target| {
                            if target == &url || target.as_str() == "about:blank" { return true; }
                            if target.scheme() == "vesper-captcha" && target.host_str() == Some("result") {
                                let result = target.query_pairs().find(|(key, _)| key == "data")
                                    .and_then(|(_, value)| serde_json::from_str::<serde_json::Value>(&value).ok());
                                let valid = result.is_some_and(|proof| proof["geetest_challenge"] == "mock-challenge" && proof["geetest_validate"] == "mock-validate" && proof["geetest_seccode"] == "mock-validate|jordan");
                                println!("native captcha page loaded; typed result received: {valid}; game requests: 0");
                                handle.exit(if valid { 0 } else { 1 });
                            }
                            false
                        }).build()?;
                    Ok(())
                })();
                if let Err(error) = result { eprintln!("captcha probe failed: {error}"); app.exit(1); }
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                eprintln!("captcha probe timed out");
                app.exit(1);
            });
            Ok(())
        })
        .run(tauri::generate_context!("tests/fixtures/cookie-probe/tauri.conf.json"))
        .expect("run isolated captcha probe");
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("This probe targets the macOS WebView transport.");
}
