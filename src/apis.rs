use actix_web::{HttpResponse, Responder, web};
use azure_identity::DefaultAzureCredential;
use azure_core::auth::TokenCredential;

use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Serialize};
use tracing_attributes::instrument;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartUploadRequest {
    #[serde(rename = "file_name")]
    pub file_name: String,
    #[serde(rename = "file_size")]
    pub file_size: u64,
    #[serde(rename = "file_hash")]
    pub file_hash: String,
}

pub struct UploadResponse {
    pub upload_id : String,
    pub chunk_size : Option<u64>,
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
        println!("Failed to get token: {:?}", err);
        return HttpResponse::InternalServerError().body("Failed to get token");
    }
    let token = response.unwrap();
    let upload_id = uuid::Uuid::new_v4().to_string();


    println!("start_upload: {:?}", req);
    HttpResponse::Ok().body("start_upload")
}

