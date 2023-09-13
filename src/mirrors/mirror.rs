use async_trait::async_trait;
use axum::response::Response;
use serde_json::de;

use crate::dist::Dist;
use crate::package::Package;

#[async_trait]
pub trait Mirror {
    async fn make_package_response(&self, package: &Package) -> Response;
    async fn check_dist(&self, dist: &Dist) -> bool;
    async fn make_dist_response(&self, dist: &Dist) -> Response;
}
