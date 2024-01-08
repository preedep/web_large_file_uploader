use actix_multipart::form::bytes::Bytes;
use actix_multipart::form::MultipartForm;
use actix_multipart::form::text::Text;
use actix_web::{HttpResponse, Responder, web};
use azure_core::auth::TokenCredential;
use azure_identity::DefaultAzureCredential;
use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};
use tracing_attributes::instrument;


#[derive(Clone, Debug, Serialize, Deserialize)]
struct UploadInfo {
    upload_id: String,
    file_name: String,
    file_size: u64,
    file_hash: String,
    blob_access_token: String,
    blob_file_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartUploadRequest {
    #[serde(rename = "file_name")]
    pub file_name: String,
    #[serde(rename = "file_size")]
    pub file_size: u64,
    #[serde(rename = "file_hash")]
    pub file_hash: String,
}

#[derive(Debug, MultipartForm)]
pub struct ContinueUploadRequest {

    pub upload_id: Text<String>,

    pub chunk_data: Option<Bytes>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub upload_id: String,
    pub chunk_size: Option<u64>,
}

type DbPool = r2d2::Pool<SqliteConnectionManager>;

#[instrument]
pub async fn start_upload(
    pool: web::Data<DbPool>,
    req: web::Json<StartUploadRequest>) -> impl Responder {
    let credential = DefaultAzureCredential::default();
    let response = credential
        .get_token(&["https://management.azure.com/.default"])
        .await;

    if let Err(err) = response {
        error!("Failed to get token: {:?}", err);
        return HttpResponse::InternalServerError().body("Failed to get token");
    }
    let token = response.unwrap();
    let upload_id = uuid::Uuid::new_v4().to_string();

    let res = pool.get().unwrap().execute(
        r#"
            INSERT INTO temp_file_uploader(
                upload_id,
                file_name,
                file_size,
                file_hash,
                blob_access_token,
                blob_file_hash
            ) VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6
            );
        "#,
        (
            &upload_id,
            &req.file_name,
            &req.file_size,
            &req.file_hash,
            &token.token.secret(),
            &"-"
        ),
    );
    if let Err(e) = res {
        error!("insert failed: {:?}", e);
        return HttpResponse::InternalServerError().body("insert failed");
    }
    let response = UploadResponse {
        upload_id,
        chunk_size: Some(1024 * 1024 * 64),
    };
    debug!("start_upload: {:?}", response);
    //println!("start_upload: {:?}", req);
    HttpResponse::Ok().json(response)
}

#[instrument(skip(form))]
pub async fn continue_upload(pool: web::Data<DbPool>,
                             form: MultipartForm<ContinueUploadRequest>) -> impl Responder {
    let update_id = &form.upload_id;
    let update_id = update_id.as_str();
    debug!("continue_upload with : {:?}", update_id);
    let res = pool.get().unwrap().query_row(
        r#"
            SELECT * FROM temp_file_uploader WHERE upload_id = ?1;
        "#,
        &[&update_id],
        |row| {
            let upload_info = UploadInfo {
                upload_id:row.get(0)?,
                file_name:row.get(1)?,
                file_size:row.get(2)?,
                file_hash:row.get(3)?,
                blob_access_token:row.get(4)?,
                blob_file_hash:row.get(5)?,
            };
            Ok(upload_info)
        },
    );
    if let Err(e) = res {
        error!("query failed: {:?}", e);
        return HttpResponse::InternalServerError().body("query failed");
    }
    let upload_info = res.unwrap();
    debug!("continue_upload: {:#?}", upload_info);

    HttpResponse::Ok().body("continue_upload")
}
#[instrument]
pub async fn finish_upload(pool: web::Data<DbPool>) -> impl Responder {
    HttpResponse::Ok().body("finish_upload")
}