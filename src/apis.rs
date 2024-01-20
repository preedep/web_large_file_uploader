use std::sync::Arc;

use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, web};
use azure_identity::DefaultAzureCredential;
use azure_storage::StorageCredentials;
use azure_storage_blobs::prelude::ClientBuilder;
use tracing::{debug, error};
use tracing_attributes::instrument;

use crate::mime_types::MIME_TYPE;
use crate::models::{
    Config, ContinueUploadRequest, DbPool, ErrorResponse, FinishResponse, FinishUploadRequest,
    MAX_CHUNK_SIZE, SharedData, StartUploadRequest, UploadInfo, UploadResponse, WebAPIResult,
};

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

    let file_ext = &req.file_name.split('.').last();
    let file_ext = match file_ext {
        Some(ext) => ext,
        None => {
            error!("file_ext not found");
            return Err(ErrorResponse::new("file_ext not found"));
        }
    };
    let content_type = MIME_TYPE
        .get(file_ext)
        .unwrap_or(&"application/octet-stream");
    debug!("start_upload content_type : {:#?}", content_type);

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
            content_type,
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
