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
pub struct Packagist{
    packages_meta_url_template: String,
}

impl Packagist{
    pub fn new() -> Self {
        Self {
            packages_meta_url_template: env::var("PACKAGES_META_URL_TEMPLATE").unwrap(),
        }
    }

    pub async fn make_package_response<'a>(&self, package: &Package<'a>) -> Response {
        let url = self
            .packages_meta_url_template
            .replace("%package%", &package.full_name);
        request_helper::proxy(&url).await
    }

    pub async fn make_dist_response<'a>(&self, dist: &Dist<'a>) -> Response {
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
