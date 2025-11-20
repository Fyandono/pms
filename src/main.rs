use actix_web::web;
use actix_web::{App, HttpServer, Responder, get, web::Data};
mod database;
mod features;
mod util;
use database::postgres::get_postgres_client;
use dotenv::dotenv;
use features::admin::services::{
    get_list_pm, get_list_project, get_list_vendor, post_create_vendor, post_create_vendor_project,
    put_edit_vendor, put_edit_vendor_project,
};
use features::vendor::services::{
    get_list_pm_u, get_list_project_u, post_create_project_pm_u, post_create_vendor_project_u,
    put_edit_project_pm_u, put_edit_vendor_project_u, put_edit_vendor_u,
};
use sqlx::{Pool, Postgres};

#[get("/index.html")]
async fn index() -> impl Responder {
    "Hello world!"
}
pub struct AppState {
    postgres: Pool<Postgres>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Starting server ...");
    dotenv().ok();

    // initiate database
    let postgres_pool = get_postgres_client().await;
    println!("🚀 Server connection to PostgreSQL success");

    HttpServer::new(move || {
        // let bearer_middleware = HttpAuthentication::bearer(validator);
        App::new()
            .app_data(Data::new(AppState {
                postgres: postgres_pool.clone(),
            }))
            .service(
                web::scope("x")
                    .service(index)
                    .service(get_list_vendor)
                    .service(get_list_project)
                    .service(get_list_pm)
                    .service(post_create_vendor)
                    .service(post_create_vendor_project)
                    .service(put_edit_vendor)
                    .service(put_edit_vendor_project),
            )
            .service(
                web::scope("u")
                    .service(put_edit_vendor_u)
                    .service(get_list_project_u)
                    .service(get_list_pm_u)
                    .service(post_create_vendor_project_u)
                    .service(put_edit_vendor_project_u)
                    .service(post_create_project_pm_u)
                    .service(put_edit_project_pm_u),
            )
        // .service(
        //     web::scope("dashboard")
        //         // .wrap(bearer_middleware)
        //         // .service(get_absen)
        // )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
