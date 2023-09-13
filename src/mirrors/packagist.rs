use axum::{
    body::{boxed, StreamBody},
    http::{HeaderMap, HeaderName, HeaderValue},
    response::{IntoResponse, Response},
};
use futures::StreamExt;
use qiniu_sdk::{
    upload::{
        apis::credential::Credential, AutoUploader, AutoUploaderObjectParams, UploadManager,
        UploadTokenSigner,
    },
    ureq::http::AsyncResponseBody,
};
use reqwest::{Client, Response as ReqwestResponse, StatusCode};
use serde_json::Value;
use std::time::Duration;

use crate::dist::Dist;
use crate::package::Package;

fn create_client() -> Client {
    Client::builder()
        .build()
        .unwrap()
}

async fn get(url: &str) -> ReqwestResponse {
    let client = create_client();

    client.get(url)
    .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36 Edg/116.0.1938.69")
    .send().await.unwrap()
}

async fn head(url: &str) -> Result<ReqwestResponse, reqwest::Error> {
    let client = create_client();

    client.head(url).send().await
}

async fn proxy(url: &str) -> Response {
    let reqwest_response = get(url).await;
    let mut response_builder = Response::builder().status(reqwest_response.status());

    *response_builder.headers_mut().unwrap() = reqwest_response.headers().clone();

    response_builder
        .body(boxed(StreamBody::new(reqwest_response.bytes_stream())))
        .unwrap()
}

fn redirect(url: &str) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("location"),
        HeaderValue::try_from(url).unwrap(),
    );
    (StatusCode::TEMPORARY_REDIRECT, headers, "").into_response()
}

#[derive(Clone)]
pub struct Packagist<'a> {
    packages_meta_url_template: &'a str,
    object_template: &'a str,
    domain: &'a str,
    access_key: &'a str,
    secret_key: &'a str,
    bucket_name: &'a str,
}

impl<'a> Packagist<'a> {
    pub fn new(
        domain: &'a str,
        access_key: &'a str,
        secret_key: &'a str,
        bucket_name: &'a str,
    ) -> Self {
        Self {
            packages_meta_url_template: "https://packagist.org/p2/%package%.json",
            object_template: "%package%/%version%/%reference%.%dist_type%",
            domain,
            access_key,
            secret_key,
            bucket_name,
        }
    }

    async fn get_origin_dist_url(&self, dist: &Dist<'a>) -> Option<String> {
        let url = self
            .packages_meta_url_template
            .replace("%package%", &dist.package.full_name);
        let res_json = get(&url).await.json::<Value>().await.unwrap();
        let mut dist_url: Option<String> = None;

        for detail in res_json["packages"][&dist.package.full_name]
            .as_array()
            .unwrap()
        {
            if detail["version"] == dist.version {
                dist_url = Some(detail["dist"]["url"].to_string());
            }
        }

        dist_url
    }

    fn get_object_name(&self, dist: &Dist<'a>) -> String {
        self.object_template
            .replace("%package%", &dist.package.full_name)
            .replace("%version%", &dist.version)
            .replace("%reference%", &dist.reference)
            .replace("%dist_type%", &dist.dist_type)
    }

    fn get_dist_url(&self, dist: &Dist<'a>) -> String {
        format!("http://{}/{}", self.domain, self.get_object_name(dist))
    }

    async fn upload(&self, origin_dist_url: &str, dist: &Dist<'a>) -> bool {
        let credential = Credential::new(self.access_key, self.secret_key);

        let upload_manager = UploadManager::builder(UploadTokenSigner::new_credential_provider(
            credential,
            self.bucket_name,
            Duration::from_secs(3600),
        ))
        .build();
        let uploader: AutoUploader = upload_manager.auto_uploader();

        let params = AutoUploaderObjectParams::builder()
            .object_name(self.get_object_name(dist))
            .file_name(self.get_object_name(dist))
            .build();
        let reqwest_response = get(origin_dist_url).await;
        let mut buffer = Vec::new();
        let bytes_read = async {
            let mut stream = reqwest_response.bytes_stream();
            while let Some(item) = stream.next().await {
                let item = item?;
                buffer.extend(item);
            }
            Ok::<_, reqwest::Error>(buffer)
        };
        let res = uploader
            .async_upload_reader(
                AsyncResponseBody::from_bytes(bytes_read.await.unwrap()),
                params,
            )
            .await;
        match res {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub async fn make_package_response(&self, package: &Package<'a>) -> Response {
        let url = self
            .packages_meta_url_template
            .replace("%package%", &package.full_name);
        proxy(&url).await
    }

    pub async fn check_dist(&self, dist: &Dist<'a>) -> bool {
        let url = self.get_dist_url(dist);
        let response = head(&url).await;
        match response {
            Ok(response) => {
                if response.status() == StatusCode::OK {
                    true
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

    pub async fn make_dist_response(&self, dist: &Dist<'a>) -> Response {
        let dist_url = self.get_dist_url(dist);
        match self.check_dist(dist).await {
            true => redirect(&dist_url),
            false => {
                let origin_dist_url = self.get_origin_dist_url(dist).await;
                match origin_dist_url {
                    Some(origin_dist_url) => {
                        match self.upload(origin_dist_url.as_str(), dist).await {
                            true => redirect(&dist_url),
                            false => (StatusCode::NOT_FOUND, HeaderMap::new(), "").into_response(),
                        }
                    }
                    None => (StatusCode::NOT_FOUND, HeaderMap::new(), "").into_response(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::env;

    #[tokio::test]
    async fn it_works() {
        let access_key = env::var("ACCESS_KEY").unwrap();
        let secret_key = env::var("SECRET_KEY").unwrap();
        let bucket_name = env::var("BUCKET").unwrap();
        let object_name =
            "tiderjian/think-core/v12.30.0/35c34ca5af137fa28b151de5b0d839d51c4a1fa9.zip";

        let credential = Credential::new(access_key, secret_key);

        let upload_manager = UploadManager::builder(UploadTokenSigner::new_credential_provider(
            credential,
            bucket_name,
            Duration::from_secs(3600),
        ))
        .build();
        let mut uploader: AutoUploader = upload_manager.auto_uploader();

        let params = AutoUploaderObjectParams::builder()
            .object_name(object_name)
            .file_name(object_name)
            .build();
        let reqwest_response = get("https://api.github.com/repos/egulias/EmailValidator/zipball/3a85486b709bc384dae8eb78fb2eec649bdb64ff").await;
        let mut buffer = Vec::new();
        let bytes_read = async {
            let mut stream = reqwest_response.bytes_stream();
            while let Some(item) = stream.next().await {
                let item = item?;
                buffer.extend(item);
            }
            Ok::<_, reqwest::Error>(buffer)
        };
        let res = uploader
            .async_upload_reader(
                AsyncResponseBody::from_bytes(bytes_read.await.unwrap()),
                params,
            )
            .await;
        println!("{:#?}", res);
        assert_eq!(Some(&vec![Value::Bool(true)]), Some(&vec![Value::Null]));
    }
}
