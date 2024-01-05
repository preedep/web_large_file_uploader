use actix_web::{HttpResponse, post, Responder};
use serde::{Deserialize, Serialize};
use tracing_attributes::instrument;

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct StartUploadRequest {
    #[serde(rename = "file_name")]
    pub file_name: String,
    #[serde(rename = "file_size")]
    pub file_size: u64,
    #[serde(rename = "file_hash")]
    pub file_hash: String
}

pub struct UploadResponse {

}

#[instrument]
pub async fn start_upload(req: actix_web::web::Json<StartUploadRequest>) -> impl Responder {
    println!("start_upload: {:?}", req);
    HttpResponse::Ok().body("start_upload")
}

