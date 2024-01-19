use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use actix_multipart::form::bytes::Bytes;
use actix_multipart::form::MultipartForm;
use actix_multipart::form::text::Text;
use actix_web::{HttpResponse, Responder, ResponseError, web};
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use azure_identity::DefaultAzureCredential;
use azure_storage::StorageCredentials;
use azure_storage_blobs::prelude::ClientBuilder;
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
    content_type: String,
    blob_access_token: String,
    blob_file_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub account: String,
    pub container: String,
}

impl Config {
    pub fn new(account: &String, container: &String) -> Config {
        Config {
            account: account.clone(),
            container: container.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartUploadRequest {
    #[serde(rename = "file_name")]
    pub file_name: String,
    #[serde(rename = "file_size")]
    pub file_size: u64,
    #[serde(rename = "file_hash")]
    pub file_hash: String,
    #[serde(rename = "content_type")]
    pub content_type: String,
}

#[derive(Debug, MultipartForm)]
pub struct ContinueUploadRequest {
    #[multipart(limit = "1KiB")]
    pub upload_id: Text<String>,
    #[multipart(limit = "128MiB")]
    pub chunk_data: Option<Bytes>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FinishUploadRequest {
    #[serde(rename = "upload_id")]
    pub upload_id: String,
}

const MAX_CHUNK_SIZE: u64 = 1024 * 1024 * 16;

#[derive(Clone, Debug, derive_more::Display, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl ErrorResponse {
    pub fn new(error: &str) -> ErrorResponse {
        ErrorResponse {
            error: error.to_string(),
        }
    }
}

impl ResponseError for ErrorResponse {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub upload_id: String,
    pub chunk_size: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinishResponse {
    #[serde(rename = "upload_id")]
    pub upload_id: String,
    #[serde(rename = "file_hash")]
    pub file_hash: String,
}

type DbPool = r2d2::Pool<SqliteConnectionManager>;

pub type WebAPIResult<T> = Result<T, ErrorResponse>;

#[derive(Debug, Clone)]
pub struct SharedData {
    pub shared_data_map: Arc<Mutex<HashMap<String, StorageCredentials>>>,
}

#[instrument]
pub async fn start_upload(
    config: web::Data<Config>,
    shared_credentials: web::Data<SharedData>,
    pool: web::Data<DbPool>,
    req: web::Json<StartUploadRequest>,
) -> WebAPIResult<impl Responder> {
    let default_creds = Arc::new(DefaultAzureCredential::default());
    let credentials = StorageCredentials::token_credential(default_creds);
    let upload_id = uuid::Uuid::new_v4().to_string();
    shared_credentials
        .shared_data_map
        .lock()
        .unwrap()
        .insert(upload_id.clone(), credentials.clone());
    let res = pool.get().unwrap().execute(
        r#"
            INSERT INTO temp_file_uploader(
                upload_id,
                file_name,
                file_size,
                file_hash,
                content_type,
                blob_access_token,
                blob_file_hash
            ) VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6,
                ?7
            );
        "#,
        (
            &upload_id,
            &req.file_name,
            &req.file_size,
            &req.file_hash,
            &req.content_type,
            &"-",
            &"-",
        ),
    );
    if let Err(e) = res {
        error!("insert failed: {:?}", e);
        return Err(ErrorResponse::new("insert failed"));
    }

    let blob_client = ClientBuilder::new(&config.account, credentials.clone())
        .blob_client(&config.container, &req.file_name);

    //let content_type = "text/plain";
    let block_res = blob_client
        .put_append_blob()
        .content_type(&req.content_type)
        .await;
    if let Err(e) = block_res {
        error!("put block failed: {:#?}", e);
        return Err(ErrorResponse::new("put block failed"));
    }

    let resp = UploadResponse {
        upload_id,
        chunk_size: Some(MAX_CHUNK_SIZE),
    };
    debug!("start_upload: {:#?}", resp);
    Ok(HttpResponse::Ok().json(resp))
}

#[instrument(skip(form))]
pub async fn continue_upload(
    shared_credentials: web::Data<SharedData>,
    config: web::Data<Config>,
    pool: web::Data<DbPool>,
    form: MultipartForm<ContinueUploadRequest>,
) -> WebAPIResult<impl Responder> {
    let update_id = &form.upload_id;
    let update_id = update_id.as_str();

    let res = pool.get().unwrap().query_row(
        r#"
            SELECT
                upload_id,
                file_name,
                file_size,
                file_hash,
                content_type,
                blob_access_token,
                blob_file_hash
            FROM temp_file_uploader WHERE upload_id = ?1;
        "#,
        &[&update_id],
        |row| {
            let upload_info = UploadInfo {
                upload_id: row.get(0)?,
                file_name: row.get(1)?,
                file_size: row.get(2)?,
                file_hash: row.get(3)?,
                content_type: row.get(4)?,
                blob_access_token: row.get(5)?,
                blob_file_hash: row.get(6)?,
            };
            Ok(upload_info)
        },
    );
    if let Err(e) = res {
        error!("query failed: {:?}", e);
        return Err(ErrorResponse::new("query failed"));
    }
    let upload_info = res.unwrap();
    match shared_credentials
        .shared_data_map
        .lock()
        .unwrap()
        .get(update_id)
    {
        Some(credentials) => {
            debug!("continue_upload credentials : {:?}", credentials);
            let blob_client = ClientBuilder::new(&config.account, credentials.to_owned())
                .blob_client(&config.container, &upload_info.file_name);
            match form.into_inner().chunk_data {
                Some(chunk_data) => {
                    //debug!("continue_upload chunk_data : {:?}", chunk_data);
                    debug!("continue_upload chunk_data : {:#?}", &chunk_data);
                    //let content_type = "text/plain";
                    //if let Some(mut mime_type) = chunk_data.content_type {
                    //    debug!("continue_upload content_type : {:#?}", mime_type);
                    //}
                    let block_res = blob_client
                        .append_block(chunk_data.data.to_vec())
                        //.content_type(content_type)
                        .await;
                    if let Err(e) = block_res {
                        error!("put block failed: {:#?}", e);
                        return Err(ErrorResponse::new("put block failed"));
                    }
                }
                None => {
                    error!("continue_upload chunk_data not found");
                    return Err(ErrorResponse::new("continue_upload chunk_data not found"));
                }
            }
        }
        None => {
            error!("continue_upload credentials not found");
            return Err(ErrorResponse::new("continue_upload credentials not found"));
        }
    }

    //TokenCredential::new(access_token);
    let resp = UploadResponse {
        upload_id: upload_info.upload_id,
        chunk_size: None,
    };
    debug!("continue_upload: {:#?}", resp);
    Ok(HttpResponse::Ok().json(resp))
}

#[instrument]
pub async fn finish_upload(
    pool: web::Data<DbPool>,
    config: web::Data<Config>,
    req: web::Json<FinishUploadRequest>,
) -> WebAPIResult<impl Responder> {
    //debug!("finish_upload with : {:#?}", req);
    let update_id = &req.upload_id;

    let resp = FinishResponse {
        upload_id: update_id.clone(),
        file_hash: "".to_string(),
    };
    Ok(HttpResponse::Ok().json(resp))
}
