use actix_files::Files;
use actix_web::{App, HttpServer, web};
use actix_web::middleware::Logger;
use actix_web::web::Data;
use log::{debug, error};
use r2d2::ManageConnection;
use r2d2_sqlite::SqliteConnectionManager;

mod apis;


//type DbPool = r2d2::Pool<r2d2::ConnectionManager<SqliteConnection>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    pretty_env_logger::init();

    let con_manager = SqliteConnectionManager::memory();
    let pool_res = r2d2::Pool::new(con_manager);
    if let Err(e) = pool_res {
        error!("create pool failed: {:?}", e);
        return Ok(());
    }
    let pool = pool_res.unwrap();
    debug!("create pool success");
    let ret = pool.get()
        .unwrap().execute(
        r#"
            BEGIN;
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
        COMMIT;
        "#,
        (), // empty list of parameters.
    );

    if let Err(e) = ret {
        error!("create table failed: {:?}", e);
        return Ok(());
    }
    debug!("create table success");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(pool.clone()))
            .wrap(Logger::default())
            .wrap(Logger::new(
                r#"%a %t "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#,
            ))
            .service(web::scope("/v1")
                .route("/start_upload", web::post().to(apis::start_upload))
            )
            .service(Files::new("statics", "./statics")
                .prefer_utf8(true)
                .index_file("index.html"))
    })
        .bind(("0.0.0.0", 8888))?
        .run()
        .await
}
