use axum::{
    http::HeaderMap,
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
use reqwest::StatusCode;
use serde_json::Value;
use std::env;
use std::time::Duration;

use crate::dist::Dist;
use crate::request_helper;

pub struct StorageSelfStrategy<'a> {
    domain: String,
    access_key: String,
    secret_key: String,
    bucket_name: String,
    object_template: String,
    packages_meta_url_template: String,
    dist_url_params: &'a Dist<'a>,
}

impl<'a> StorageSelfStrategy<'a> {
    pub fn new(dist: &'a Dist<'a>, packages_meta_url_template: String) -> Self {
        let domain = env::var("DOMAIN").unwrap();
        let access_key = env::var("ACCESS_KEY").unwrap();
        let secret_key = env::var("SECRET_KEY").unwrap();
        let bucket_name = env::var("BUCKET").unwrap();

        Self {
            domain,
            access_key,
            secret_key,
            bucket_name,
            object_template: String::from("%package%/%version%/%reference%.%dist_type%"),
            dist_url_params: dist,
            packages_meta_url_template,
        }
    }

    async fn get_origin_dist_url(&self) -> Option<String> {
        let url = self
            .packages_meta_url_template
            .replace("%package%", &self.dist_url_params.package.full_name);
        let res_json = request_helper::get(&url)
            .await
            .json::<Value>()
            .await
            .unwrap();
        let mut dist_url: Option<String> = None;

        for detail in res_json["packages"][&self.dist_url_params.package.full_name]
            .as_array()
            .unwrap()
        {
            if detail["version"] == self.dist_url_params.version {
                dist_url = Some(detail["dist"]["url"].to_string());
            }
        }

        dist_url
    }

    fn get_object_name(&self) -> String {
        self.object_template
            .replace("%package%", &self.dist_url_params.package.full_name)
            .replace("%version%", self.dist_url_params.version)
            .replace("%reference%", self.dist_url_params.reference)
            .replace("%dist_type%", self.dist_url_params.dist_type)
    }

    fn get_dist_url(&self) -> String {
        format!("http://{}/{}", self.domain, self.get_object_name())
    }

    async fn check_dist(&self) -> bool {
        let url = self.get_dist_url();
        let response = request_helper::head(&url).await;
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

    async fn upload(&self, origin_dist_url: &str) -> bool {
        let credential = Credential::new(&self.access_key, &self.secret_key);

        let upload_manager = UploadManager::builder(UploadTokenSigner::new_credential_provider(
            credential,
            &self.bucket_name,
            Duration::from_secs(3600),
        ))
        .build();
        let uploader: AutoUploader = upload_manager.auto_uploader();

        let params = AutoUploaderObjectParams::builder()
            .object_name(self.get_object_name())
            .file_name(self.get_object_name())
            .build();
        let reqwest_response = request_helper::get(origin_dist_url).await;
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

    pub async fn run(&self) -> Response {
        let dist_url = self.get_dist_url();
        match self.check_dist().await {
            true => request_helper::redirect(&dist_url),
            false => match self.get_origin_dist_url().await {
                Some(origin_dist_url) => match self.upload(&origin_dist_url).await {
                    true => request_helper::redirect(&dist_url),
                    false => (StatusCode::NOT_FOUND, HeaderMap::new(), "").into_response(),
                },
                None => (StatusCode::NOT_FOUND, HeaderMap::new(), "").into_response(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn qiniu_upload_test() {
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
        let uploader: AutoUploader = upload_manager.auto_uploader();

        let params = AutoUploaderObjectParams::builder()
            .object_name(object_name)
            .file_name(object_name)
            .build();
        let reqwest_response = request_helper::get("https://api.github.com/repos/quansitech/think-core/zipball/35c34ca5af137fa28b151de5b0d839d51c4a1fa9").await;
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

    #[tokio::test]
    async fn proxy_test() {
        let url =
            "https://packagist.kr/p2/%package%.json".replace("%package%", "tiderjian/think-core");
        let client = request_helper::create_client();

        let reqwest_response = client.get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36 Edg/116.0.1938.69")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .header("Cache-Control", "no-cache")
            .header("Pragma", "no-cache")
            .send().await.unwrap();

        println!("{:#?}", reqwest_response.headers());

        assert_eq!(1, 2);
    }
}
