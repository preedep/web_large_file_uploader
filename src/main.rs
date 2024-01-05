use actix_files::Files;
use actix_web::{App, HttpServer, web};
use actix_web::middleware::Logger;
use actix_web::web::Data;
use log::error;
use rusqlite::Connection;

mod apis;


//type DbPool = r2d2::Pool<r2d2::ConnectionManager<SqliteConnection>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    pretty_env_logger::init();

    let conn = Connection::open_in_memory();
    if let Err(e) = conn {
        error!("open_in_memory failed: {:?}", e);
        return Ok(());
    }
    let conn = conn.unwrap();
    let ret = conn.execute(
        "CREATE TABLE  temp_file_uploader(
            id   INTEGER PRIMARY KEY,
            uuid TEXT NOT NULL UNIQUE,
            file_name TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            file_hash TEXT NOT NULL
        )",
        (), // empty list of parameters.
    );
    if let Err(e) = ret {
        error!("create table failed: {:?}", e);
        return Ok(());
    }
    HttpServer::new(|| {
        App::new()
            //.app_data(Data::new(conn))
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
