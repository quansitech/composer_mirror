use axum::{
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use reqwest::StatusCode;
use std::env;
use serde_json::Value;

use tokio::task;
use tokio::select;

use crate::{dist::Dist, mirrors::tencent::Tencent, mirrors::aliyun::Aliyun};
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
        let mut urls = Vec::new();
        let source_url = self.get_tag_url().await;
        for site in self.cache_site_list.iter() {
            let url = format!("{}/{}", site, source_url);
            urls.push(url);
        }

        let tencent_mirror = Tencent::new();
        urls.push(tencent_mirror.get_dist_url(&self.dist_url_params));

        let aliyun_mirror = Aliyun::new();
        urls.push(aliyun_mirror.get_dist_url(&self.dist_url_params));
        
        let mut tasks = Vec::new();
        for url in urls {
            let task = task::spawn(request_helper::speed_test(url));
            tasks.push(task);
        }

        let res;
        loop{
            select!(
                result = futures::future::select_all(tasks) => {
                    let (finished_result, _, remaining_tasks) = result;
                    if remaining_tasks.len() == 0 {
                        res = (StatusCode::NOT_FOUND, HeaderMap::new(), "").into_response();
                        break;
                    }

                    tasks = remaining_tasks;

                    if let Ok(Some((url, _))) = finished_result {
                        res = request_helper::redirect(&url);
                        break;
                    }
                }
            )
        }
        res
    }
}
