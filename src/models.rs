use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use actix_multipart::form::bytes::Bytes;
use actix_multipart::form::MultipartForm;
use actix_multipart::form::text::Text;
use actix_web::{HttpResponse, ResponseError};
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use azure_storage::StorageCredentials;
use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadInfo {
    pub upload_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_hash: String,
    pub content_type: String,
    pub blob_access_token: String,
    pub blob_file_hash: String,
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

pub const MAX_CHUNK_SIZE: u64 = 1024 * 1024 * 16;

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

pub type DbPool = r2d2::Pool<SqliteConnectionManager>;

pub type WebAPIResult<T> = Result<T, ErrorResponse>;

#[derive(Debug, Clone)]
pub struct SharedData {
    pub shared_data_map: Arc<Mutex<HashMap<String, StorageCredentials>>>,
}
