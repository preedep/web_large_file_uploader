use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use actix_files::Files;
use actix_web::{App, HttpServer, web};
use actix_web::middleware::Logger;
use actix_web::web::Data;
use log::{debug, error};
use r2d2_sqlite::SqliteConnectionManager;

use crate::apis::Config;

mod apis;

//type DbPool = r2d2::Pool<r2d2::ConnectionManager<SqliteConnection>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    pretty_env_logger::init();

    let account = std::env::var("STORAGE_ACCOUNT").expect("missing STORAGE_ACCOUNT");
    let container = std::env::var("STORAGE_CONTAINER").expect("missing STORAGE_CONTAINER");
    //let blob_name = std::env::var("STORAGE_BLOB_NAME").expect("missing STORAGE_BLOB_NAME");

    let config = Config::new(&account, &container);

    let con_manager = SqliteConnectionManager::memory();
    let pool_res = r2d2::Pool::new(con_manager);
    if let Err(e) = pool_res {
        error!("create pool failed: {:?}", e);
        return Ok(());
    }
    let pool = pool_res.unwrap();
    debug!("create pool success");
    let ret = pool.get().unwrap().execute(
        r#"
            CREATE TABLE  temp_file_uploader(
            id   INTEGER PRIMARY KEY,
            upload_id TEXT NOT NULL UNIQUE,
            file_name TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            file_hash TEXT NOT NULL,
            blob_access_token TEXT NOT NULL,
            blob_file_hash TEXT NOT NULL,
            created_dt DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX temp_file_uploader_idxs ON temp_file_uploader(upload_id);
        "#,
        (), // empty list of parameters.
    );

    if let Err(e) = ret {
        error!("create table failed: {:?}", e);
        return Ok(());
    }
    debug!("create table success");

    let shared_credentails = Data::new(apis::SharedData {
        shared_data_map: Arc::new(Mutex::new(HashMap::new())),
    });
    HttpServer::new(move || {
        App::new()
            .app_data(shared_credentails.clone())
            //.app_data(Data::new(PayloadConfig::new(128 * 1024 * 1024).clone()))
            .app_data(Data::new(config.clone()))
            .app_data(Data::new(pool.clone()))
            .wrap(Logger::default())
            .wrap(Logger::new(
                r#"%a %t "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#,
            ))
            .service(
                web::scope("/api/v1")
                    .route("/start_upload", web::post().to(apis::start_upload))
                    .route("/continue_upload", web::post().to(apis::continue_upload))
                    .route("/finish_upload", web::post().to(apis::finish_upload)),
            )
            .service(
                Files::new("statics", "./statics")
                    .prefer_utf8(true)
                    .index_file("index.html"),
            )
    })
        .bind(("0.0.0.0", 8888))?
        .run()
        .await
}
