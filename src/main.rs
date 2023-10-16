use axum::{
    extract::Path,
    http::{HeaderMap, HeaderName, HeaderValue},
    response::{Html, IntoResponse, Response},
    routing::get,
    Extension, Router,
};
use glob::Pattern;
use std::{sync::Arc};

use dotenv::dotenv;
use reqwest::StatusCode;
use std::env;
use std::fs::OpenOptions;
use std::io::prelude::*;

mod dist;
mod mirrors;
mod package;
mod request_helper;

use crate::dist::Dist;
use crate::mirrors::aliyun::Aliyun;
use crate::mirrors::packagist::Packagist;
use crate::mirrors::tencent::Tencent;
use crate::package::Package;

#[derive(Clone)]
struct Config {
    packages: String,
    package_white_list: Vec<String>
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let mut packages = String::new();
    let mut packages_file = OpenOptions::new()
        .read(true)
        .open("./packages.json")
        .unwrap();

    packages_file.read_to_string(&mut packages).unwrap();

    let package_white_list = env::var("PACKAGE_WHITE_LIST")
        .unwrap()
        .split(",")
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    let config = Arc::new(Config {
        packages,
        package_white_list
    });

    let app = Router::new()
        .route("/p2/*package_path", get(package_meta))
        .route(
            "/dists/:package1/:package2/:version/:reference_and_type",
            get(dist_dispatcher),
        )
        .route("/packages.json", get(packages_meta))
        .layer(Extension(config));

    let listen = format!("0.0.0.0:{}", env::var("PORT").unwrap());
    axum::Server::bind(&listen.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn dist_dispatcher<'a>(
    Path((package1, package2, version, reference_and_type)): Path<(String, String, String, String)>,
    config: Extension<Arc<Config>>,
) -> Response {
    let reference = reference_and_type.split(".").collect::<Vec<&str>>()[0];
    let dist_type = reference_and_type.split(".").collect::<Vec<&str>>()[1];
    let package = Package::new(&package1, &package2);
    let dist = Dist::new(&package, &version, reference, dist_type);

    let packagist_mirror = Packagist::new();
    let tenecnt_mirror = Tencent::new();
    let aliyun_mirror = Aliyun::new();

    match check_package_in_white_list(&format!("{}/{}", package1, package2), &config.package_white_list) {
        true => {
            packagist_mirror.make_dist_response(&dist).await
        }
        false => {
            match tenecnt_mirror.check_dist(&dist).await {
                true => tenecnt_mirror.make_dist_response(&dist).await,
                false => match aliyun_mirror.check_dist(&dist).await {
                    true => aliyun_mirror.make_dist_response(&dist).await,
                    false => packagist_mirror.make_dist_response(&dist).await,
                }
            }
        }
    }
}

async fn packages_meta<'a>(
    config: Extension<Arc<Config>>,
) -> (StatusCode, HeaderMap, Html<String>) {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_str("application/json").unwrap(),
    );

    (StatusCode::OK, headers, Html(config.packages.clone()))
}

async fn package_meta<'a>(
    Path(package_path): Path<String>,
    config: Extension<Arc<Config>>,
) -> Response {
    let headers = HeaderMap::new();
    if !package_path.ends_with(".json") {
        return (StatusCode::NOT_FOUND, headers, "").into_response();
    }

    let package_combine = package_path.trim_end_matches(".json");
    let vendor = package_combine.split("/").collect::<Vec<&str>>()[0];
    let package = package_combine.split("/").collect::<Vec<&str>>()[1];

    match check_package_in_white_list(&package_combine, &config.package_white_list) {
        true => {
            Packagist::new()
                .make_package_response(&Package::new(vendor, package))
                .await
        }
        false => {
            Tencent::new()
                .make_package_response(&Package::new(vendor, package))
                .await
        }
    }
}

fn check_package_in_white_list(package: &str, white_list: &Vec<String>) -> bool {
    for pattern in white_list {
        if Pattern::new(pattern).unwrap().matches(package) {
            return true;
        }
    }
    false
}