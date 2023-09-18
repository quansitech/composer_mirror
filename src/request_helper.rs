use axum::{
    http::{HeaderMap, HeaderName, HeaderValue},
    response::{IntoResponse, Response},
};
use reqwest::{Client, Response as ReqwestResponse, StatusCode};

pub fn create_client() -> Client {
    Client::builder().build().unwrap()
}

pub async fn get(url: &str) -> ReqwestResponse {
    let client = create_client();

    client.get(url)
    .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36 Edg/116.0.1938.69")
    .send().await.unwrap()
}

pub async fn head(url: &str) -> Result<ReqwestResponse, reqwest::Error> {
    let client = create_client();

    client.head(url).send().await
}

pub async fn proxy(url: &str) -> Response {
    let reqwest_response = get(url).await;

    let resp_headers = reqwest_response.headers().clone();
    let body = reqwest_response.text().await.unwrap();

    (StatusCode::OK, resp_headers, body).into_response()
}

pub fn redirect(url: &str) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("location"),
        HeaderValue::try_from(url).unwrap(),
    );
    (StatusCode::TEMPORARY_REDIRECT, headers, "").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn head_test() {
        let url = "https://ghps.cc/https://github.com/quansitech/think-core/archive/refs/tags/v12.30.0.zip";
        let response = head(url).await;
        println!("{:#?}", response);
        assert_eq!(1, 2);
    }
}
