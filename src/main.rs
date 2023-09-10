use axum::{
    routing::get,
    body::{boxed, StreamBody},
    extract::{State, Path, RawQuery, OriginalUri},
    response::{Response, Html, IntoResponse},
    Router, http::{HeaderMap, HeaderName, HeaderValue},
};
use reqwest::{Client, StatusCode};
use futures_util::stream;
use std::fs::{OpenOptions, File};
use std::io::prelude::*;
use glob::Pattern;

mod package;
mod dist;
mod mirrors;

#[derive(Clone)]
struct Config<'a> {
    packages: String,
    tencent_package_url: &'a str,
    packagist_package_url: &'a str,
    package_white_list: Vec<&'a str>
}

#[tokio::main]
async fn main() {

    let mut packages = String::new();
    let mut packages_file = OpenOptions::new()
                                    .read(true)
                                    .open("./packages.json").unwrap();

    packages_file.read_to_string(&mut packages).unwrap();

    let config = Config {
        packages,
        tencent_package_url: "https://mirrors.cloud.tencent.com/repository/composer/p/%package%.json",
        packagist_package_url: "https://packagist.org/p2/%package%.json",
        package_white_list: vec![
            "tiderjian/*", 
            "quansitech/*"
        ]
    };

    

    // build our application with a single route
    let app = Router::new()
        .route("/*path", get(redirect))
        //.route("/demo/:package", get(get_package).with_state(&config).with_state(client))
        .route("/p2/*package_path", get(package_meta).with_state(config.clone()))
        .route("/dists/:package1/:package2/:version/:reference_and_type", get(dist_dispatcher))
        .route("/packages.json", get(packages_meta).with_state(config.packages.clone()));
        

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// async fn get_package<'a>(OriginalUri(original_uri): OriginalUri, State(config): State<&Config<'a>>, State(client): State<Client>) -> Response {
//     let reqwest_response = match client.get(format!("{}{}", "https://mirrors.aliyun.com", original_uri))
//         .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36 Edg/116.0.1938.69")
//         .send().await {
//             Ok(res) => res,
//             Err(err) => {
//                 panic!("Error: {}", err)
//             }
//     };

//     let mut response_builder = Response::builder().status(reqwest_response.status());


//     *response_builder.headers_mut().unwrap() = reqwest_response.headers().clone();

//     response_builder
//         .body(boxed(StreamBody::new(reqwest_response.bytes_stream())))
//         .unwrap()
// }

// async fn demo_path(OriginalUri(original_uri): OriginalUri) -> Response{
//     Response::new(boxed(original_uri.path().to_string()))
// }

async fn dist_dispatcher(Path((package1, package2, version, reference_and_type)): Path<(String, String, String, String)>) -> &'static str {
    let new_uri = format!("/dists/{}/{}/{}/{}", package1, package2, version, reference_and_type);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("./log.txt").unwrap();

    write!(file, "{}\n", new_uri.clone()).unwrap();
    
    "完成"
}

async fn packages_meta<'a>(State(packages): State<String>) -> (StatusCode, HeaderMap, Html<String>) {
    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_str("application/json").unwrap());

    (StatusCode::OK, headers, Html(packages))
}

async fn package_meta<'a>(Path(package_path): Path<String>, State(config): State<Config<'a>>) -> Response{
    let mut headers = HeaderMap::new();
    if !package_path.ends_with(".json") {
        return (StatusCode::NOT_FOUND, headers, "").into_response();
    }

    let package = package_path.trim_end_matches(".json");

    match check_package_in_white_list(&package, &config.package_white_list){
         true => {
            let client = reqwest::Client::builder()
                                 .proxy(reqwest::Proxy::https("http://127.0.0.1:10809").unwrap())
                                 .build().unwrap();

            let reqwest_response = client.get(get_package_url(package, config.packagist_package_url))
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36 Edg/116.0.1938.69")
                .send().await.unwrap();

            let mut response_builder = Response::builder().status(reqwest_response.status());


            *response_builder.headers_mut().unwrap() = reqwest_response.headers().clone();

            response_builder
                .body(boxed(StreamBody::new(reqwest_response.bytes_stream())))
                .unwrap()
        },
         false => {
            headers.insert(HeaderName::from_static("location"), HeaderValue::try_from(get_package_url(package, config.tencent_package_url)).unwrap());
            (StatusCode::TEMPORARY_REDIRECT, headers, "").into_response()
        }
    }

    
    

    
}

async fn redirect(OriginalUri(original_uri): OriginalUri) -> (StatusCode, HeaderMap, ()){
    let new_uri = format!("{}{}", "https://mirrors.aliyun.com", original_uri);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("./log.txt").unwrap();

    write!(file, "{}\n", new_uri.clone()).unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("location"), HeaderValue::from_str(&new_uri).unwrap());
    (StatusCode::TEMPORARY_REDIRECT, headers, ())
}

fn get_package_url<'a>(package: &'a str, package_pattern_url: &'a str) -> String {
    package_pattern_url.replace("%package%", package)
}

fn check_package_in_white_list(package: &str, white_list: &Vec<&str>) -> bool {
    for pattern in white_list {
        if Pattern::new(pattern).unwrap().matches(package) {
            return true;
        }
    }
    false
}