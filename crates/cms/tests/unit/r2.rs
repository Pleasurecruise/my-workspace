use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn publishes_media_with_content_types_and_file_contents() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for content_type in ["audio/mpeg", "video/mp4"] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = socket.read(&mut buffer).await.unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&buffer[..count]);
                if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                    if request.len() < end + 4 + length {
                        continue;
                    }
                    assert!(headers.contains(&format!("content-type: {content_type}\r\n")));
                    assert!(
                        request[end + 4..]
                            .windows(b"synthetic media".len())
                            .any(|bytes| bytes == b"synthetic media")
                    );
                    break;
                }
            }
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
        }
    });
    let configuration = aws_sdk_s3::config::Builder::new()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("auto"))
        .credentials_provider(Credentials::new("test", "test", None, None, "test"))
        .endpoint_url(endpoint)
        .force_path_style(true)
        .build();
    let store = Store {
        client: Client::from_conf(configuration),
    };
    for extension in ["mp3", "mp4"] {
        let path = std::env::temp_dir().join(format!(
            "vesper-media-upload-{}.{extension}",
            std::process::id()
        ));
        tokio::fs::write(&path, b"synthetic media").await.unwrap();
        tokio::time::timeout(
            Duration::from_secs(10),
            store.put_file(&format!("test.{extension}"), &path),
        )
        .await
        .unwrap()
        .unwrap();
        tokio::fs::remove_file(&path).await.unwrap();
    }
    server.await.unwrap();
}
