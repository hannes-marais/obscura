use std::sync::Arc;
use obscura_net::{CookieJar, ObscuraHttpClient, ObscuraNetError, ResourceRequest, ResourceType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;

async fn encode(coding: &str, body: &[u8]) -> Vec<u8> {
    use async_compression::tokio::write::{BrotliEncoder, GzipEncoder, ZlibEncoder};
    match coding {
        "gzip" => {
            let mut encoder = GzipEncoder::new(Vec::new());
            encoder.write_all(body).await.unwrap();
            encoder.shutdown().await.unwrap();
            encoder.into_inner()
        }
        "br" => {
            let mut encoder = BrotliEncoder::new(Vec::new());
            encoder.write_all(body).await.unwrap();
            encoder.shutdown().await.unwrap();
            encoder.into_inner()
        }
        "deflate" => {
            let mut encoder = ZlibEncoder::new(Vec::new());
            encoder.write_all(body).await.unwrap();
            encoder.shutdown().await.unwrap();
            encoder.into_inner()
        }
        _ => unreachable!(),
    }
}

async fn fixture(coding: &str, body: Vec<u8>) -> Url {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let header = format!("HTTP/1.1 200 OK\r\nContent-Encoding: {coding}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0u8; 4096];
        let _ = socket.read(&mut request).await;
        let _ = socket.write_all(header.as_bytes()).await;
        let _ = socket.write_all(&body).await;
    });
    Url::parse(&format!("http://{address}/")).unwrap()
}

fn client() -> ObscuraHttpClient {
    ObscuraHttpClient::with_full_options(Arc::new(CookieJar::new()), None, true)
}

#[tokio::test]
async fn compressed_response_preserves_headers_and_decodes_body() {
    let text = b"same decoded bytes with representation metadata";
    for coding in ["gzip", "br", "deflate"] {
        let wire = encode(coding, text).await;
        let length = wire.len().to_string();
        let url = fixture(coding, wire).await;
        let response = client().fetch(&url).await.unwrap();
        assert_eq!(response.body, text);
        assert_eq!(response.headers.get("content-encoding"), Some(&coding.to_string()));
        assert_eq!(response.headers.get("content-length"), Some(&length));
    }
    let wire = encode("deflate", &encode("gzip", text).await).await;
    let url = fixture("gzip, deflate", wire).await;
    assert_eq!(client().fetch(&url).await.unwrap().body, text);
}

#[tokio::test]
async fn compressed_response_expansion_is_limited() {
    let text = vec![b'a'; 1 << 20];
    for coding in ["gzip", "br", "deflate"] {
        let url = fixture(coding, encode(coding, &text).await).await;
        let request = ResourceRequest::subresource(ResourceType::Image, &url)
            .with_max_response_bytes(1024);
        let error = client().fetch_resource_with_callbacks(&url, request, None).await.unwrap_err();
        assert!(matches!(error, ObscuraNetError::ResponseTooLarge { limit: 1024, .. }));
    }
}

#[tokio::test]
async fn compressed_response_empty_and_malformed_bodies() {
    let url = fixture("gzip", Vec::new()).await;
    assert!(client().fetch(&url).await.unwrap().body.is_empty());
    let url = fixture("gzip", b"malformed gzip".to_vec()).await;
    assert!(matches!(client().fetch(&url).await.unwrap_err(), ObscuraNetError::Network(_)));
}
