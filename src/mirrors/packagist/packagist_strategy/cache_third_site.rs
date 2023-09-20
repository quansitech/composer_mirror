use axum::{
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use reqwest::StatusCode;
use std::env;
use serde_json::Value;

use crate::dist::Dist;
use crate::request_helper;

pub struct CacheThirdSiteStrategy<'a> {
    cache_site_list: Vec<String>,
    dist_url_params: &'a Dist<'a>,
    zip_template: String,
    packages_meta_url_template: String
}

impl<'a> CacheThirdSiteStrategy<'a> {
    pub fn new(dist: &'a Dist<'a>, packages_meta_url_template: String) -> Self {
        let cache_site_list = env::var("CACHE_SITE_LIST")
            .unwrap()
            .split(",")
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        Self {
            zip_template: String::from(
                "%source%/archive/refs/tags/%version%.%dist_type%",
            ),
            cache_site_list,
            dist_url_params: dist,
            packages_meta_url_template
        }
    }

    async fn get_source_url(&self) -> Option<String> {
        let url = self
            .packages_meta_url_template
            .replace("%package%", &self.dist_url_params.package.full_name);
        let res_json = request_helper::get(&url)
            .await
            .json::<Value>()
            .await
            .unwrap();
        let mut source_url: Option<String> = None;

        for detail in res_json["packages"][&self.dist_url_params.package.full_name]
            .as_array()
            .unwrap()
        {
            if detail["version"] == self.dist_url_params.version {
                source_url = Some(detail["source"]["url"].to_string().replace("\"", "").replace(".git", ""));
            }
        }

        source_url
    }

    async fn get_tag_url(&self) -> String {
        self.zip_template
            .replace("%source%", &self.get_source_url().await.unwrap())
            .replace("%version%", self.dist_url_params.version)
            .replace("%dist_type%", self.dist_url_params.dist_type)
    }


    pub async fn run(&self) -> Response {
        for site in self.cache_site_list.iter() {
            let url = format!("{}/{}", site, self.get_tag_url().await);
            let response = request_helper::head(&url).await;

            match response {
                Ok(response) => {
                    if response.status() == StatusCode::OK {
                        return request_helper::redirect(&url);
                    }
                }
                Err(_) => {}
            }
        }
        (StatusCode::NOT_FOUND, HeaderMap::new(), "").into_response()
    }
}
