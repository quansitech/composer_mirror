use axum::{
    http::{HeaderMap, HeaderName, HeaderValue},
    response::{IntoResponse, Response},
};
use reqwest::{Client, StatusCode};

use crate::dist::Dist;
use crate::package::Package;

#[derive(Clone)]
pub struct Tencent<'a> {
    packages_meta_url_template: &'a str,
    dist_url_template: &'a str,
}

impl<'a> Tencent<'a> {
    pub fn new() -> Self {
        Self {
            packages_meta_url_template: "https://mirrors.cloud.tencent.com/repository/composer/p/%package%.json",
            dist_url_template: "https://mirrors.cloud.tencent.com/repository/composer/%package%/%version%/%combine%.%dist_type%"
        }
    }

    pub fn get_dist_url(&self, dist: &Dist) -> String {
        let combine = format!("{}/{}", dist.package.full_name, dist.version).replace("/", "-");
        self.dist_url_template
            .replace("%package%", &dist.package.full_name)
            .replace("%version%", &dist.version)
            .replace("%combine%", &combine)
            .replace("%dist_type%", &dist.dist_type)
    }

    pub async fn make_package_response(&self, package: &Package<'a>) -> Response {
        let url = self
            .packages_meta_url_template
            .replace("%package%", &package.full_name);
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("location"),
            HeaderValue::try_from(url).unwrap(),
        );
        (StatusCode::TEMPORARY_REDIRECT, headers, "").into_response()
    }

    pub async fn check_dist(&self, dist: &Dist<'a>) -> bool {
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
            }
            Err(_) => false,
        }
    }

    pub async fn make_dist_response(&self, dist: &Dist<'a>) -> Response {
        let url = self.get_dist_url(dist);
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("location"),
            HeaderValue::try_from(url).unwrap(),
        );
        (StatusCode::TEMPORARY_REDIRECT, headers, "").into_response()
    }
}
