mod database;
mod features;
mod util;
use std::env;

use actix_cors::Cors;
use actix_web::web;
use actix_web::{App, HttpServer, Responder, get, web::Data};
use actix_web_httpauth::middleware::HttpAuthentication;
use dotenv::dotenv;
use features::vendor_project::services::{
    get_list_pm, get_list_project, get_list_vendor, post_create_vendor, post_create_vendor_project,
    put_edit_vendor, put_edit_vendor_project, put_edit_verify_pm, get_all_vendor,
    post_create_project_pm, put_edit_project_pm, get_project_pm_file,
    get_detail_pm, get_report
};
use features::unit::services::{
    get_unit, post_create_unit, put_edit_unit
};
use features::role::services::{
    get_role, post_create_role, put_edit_role
};
use features::user::services::{register, login, update_user, get_user, change_password};
use sqlx::{Pool, MySql};
use util::jwt_validator::validate_jwt;

use crate::database::mysql::get_mysql_client;

#[get("/index.html")]
async fn index() -> impl Responder {
    "Hello world!"
}
pub struct AppState {
    db: Pool<MySql>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Starting server ...");
    dotenv().ok();

    let port_str = env::var("RUST_APP_PORT").unwrap_or_else(|_| "8080".to_string());
    let port: u16 = port_str.parse().expect("RUST_APP_PORT");

    let mysql_pool = get_mysql_client().await;
    println!("🚀 Server connection to MySQL success");
    
    // bearer
    let bearer_middleware = HttpAuthentication::bearer(validate_jwt);

    HttpServer::new(move || {

    let cors = Cors::default()
        .allow_any_origin()
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allowed_headers(vec![
            actix_web::http::header::CONTENT_TYPE,
            actix_web::http::header::ACCEPT,
            actix_web::http::header::AUTHORIZATION,
            actix_web::http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        ])
        .supports_credentials()
        .max_age(3600);

        App::new()
            .app_data(Data::new(AppState {
                db: mysql_pool.clone(),
            }))
            .wrap(cors)
            .service(index)
            .service(login)
            .service(
                web::scope("x")
                    .wrap(bearer_middleware.clone())
                    // user
                    .service(register)
                    .service(update_user)
                    .service(get_user)
                    .service(change_password)
                    
                    // vendor
                    .service(get_list_vendor)
                    .service(get_all_vendor)
                    .service(post_create_vendor)
                    .service(put_edit_vendor)

                    // project
                    .service(get_list_project)
                    .service(post_create_vendor_project)
                    .service(put_edit_vendor_project)

                    // pm
                    .service(get_list_pm)
                    .service(put_edit_verify_pm)
                    .service(post_create_project_pm)
                    .service(put_edit_project_pm)
                    .service(get_detail_pm)
                    .service(get_project_pm_file)

                    // unit
                    .service(get_unit)
                    .service(post_create_unit)
                    .service(put_edit_unit)

                    // role
                    .service(get_role)
                    .service(post_create_role)
                    .service(put_edit_role)

                    // report
                    .service(get_report)
                    ,
            )
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
