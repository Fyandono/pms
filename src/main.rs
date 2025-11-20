use actix_web::{App, HttpServer, Responder, get, web::Data};
mod database;
mod features;
mod util;
use database::postgres::get_postgres_client;
use dotenv::dotenv;
use features::admin::services::{
    get_list_pm, 
    get_list_project, 
    get_list_vendor, 
    get_dropdown_vendor,
    post_create_vendor,
    post_create_vendor_project,
    put_edit_vendor,
    put_edit_vendor_project
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
            .service(index)
            .service(get_list_vendor)
            .service(get_dropdown_vendor)
            .service(get_list_project)
            .service(get_list_pm)
            .service(post_create_vendor)
            .service(post_create_vendor_project)
            .service(put_edit_vendor)
            .service(put_edit_vendor_project)
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
