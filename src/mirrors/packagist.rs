use axum::{response::{
    Response, IntoResponse
    }, 
    http::{
        HeaderMap, HeaderName, HeaderValue
    }
};
use reqwest::{StatusCode, Client, Proxy, Response as ReqwestResponse};
use async_trait::async_trait;
use serde_json::Value;

use super::mirror::Mirror;
use crate::package::Package;
use crate::dist::Dist;

async fn get(url: &str) -> ReqwestResponse {
    let client = Client::builder()
        .proxy(Proxy::https("http://127.0.0.1:10809").unwrap())
        .build().unwrap();

    client.get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36 Edg/116.0.1938.69")
        .send().await.unwrap()
}

pub struct Packagist<'a> {
    packages_meta_url_template: &'a str
}

impl<'a> Packagist<'a> {
    pub fn new() -> Self {
        Self {
            packages_meta_url_template: "https://packagist.org/p2/%package%.json"
        }
    }

    async fn get_dist_url(&self, dist: &Dist) -> String {
        let url = self.packages_meta_url_template.replace("%package%", &dist.package.full_name);
        let res_json = get(&url).await.json::<Value>().await.unwrap();
        let dist_url = res_json["packages"][&dist.package.full_name];
    }
}

#[async_trait]
impl<'a> Mirror for Packagist<'a>{

    async fn make_package_response(&self, package: &Package) -> Response {
        let url = self.packages_meta_url_template.replace("%package%", &package.full_name);
        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_static("location"), HeaderValue::try_from(url).unwrap());
        (StatusCode::TEMPORARY_REDIRECT, headers, "").into_response()
    }

    async fn check_dist(&self, dist: &Dist) -> bool {
        let url = self.get_dist_url(dist);
        let client = Client::new();
        let response = client.head(&url).send().await;
        match response {
            Ok(response) => {
                if response.status() == StatusCode::OK {
                    true
                } else {
                    false
                }
            },
            Err(_) => false
        }
    }

    async fn make_dist_response(&self, dist: &Dist) -> Response {
        let url = self.get_dist_url(dist);
        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_static("location"), HeaderValue::try_from(url).unwrap());
        (StatusCode::TEMPORARY_REDIRECT, headers, "").into_response()
    }

}