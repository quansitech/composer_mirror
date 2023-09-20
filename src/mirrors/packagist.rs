use axum::response::Response;

use std::env;

mod packagist_strategy;

use crate::dist::Dist;
use crate::package::Package;
use crate::request_helper;

use self::packagist_strategy::{
    cache_third_site::CacheThirdSiteStrategy, storage_self::StorageSelfStrategy,
};

#[derive(Clone)]
pub struct Packagist<'a> {
    packages_meta_url_template: &'a str,
}

impl<'a> Packagist<'a> {
    pub fn new() -> Self {
        Self {
            packages_meta_url_template: "https://packagist.kr/p2/%package%.json",
        }
    }

    pub async fn make_package_response(&self, package: &Package<'a>) -> Response {
        let url = self
            .packages_meta_url_template
            .replace("%package%", &package.full_name);
        request_helper::proxy(&url).await
    }

    pub async fn make_dist_response(&self, dist: &Dist<'a>) -> Response {
        let strategy: i32 = env::var("PACKAGIST_STRATEGY").unwrap().parse().unwrap();

        match strategy {
            1 => {
                StorageSelfStrategy::new(dist, self.packages_meta_url_template.to_string())
                    .run()
                    .await
            }
            2 => CacheThirdSiteStrategy::new(dist, self.packages_meta_url_template.to_string()).run().await,
            _ => panic!("Unknown strategy"),
        }
    }
}
