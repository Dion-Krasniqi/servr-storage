use axum::{extract, Json};
use axum_extra::extract::Multipart;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use bytes::Bytes;
use std::time::Duration;

// clean these imports us
use aws_sdk_s3 as s3;
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::presigning::PresigningConfig;

use axum::extract::State;
use failure;
use axum::response::IntoResponse;

use crate::models::{DatabaseFile,
                    FileResponse,
                    OwnerId, 
                    CreateFolderForm, 
                    FileType, 
                    DeleteFileForm, 
                    AppState,
                    GetFilesError};

async fn check_bucket(client: &s3::Client, bucket_name: &str)->Result<bool, s3::Error>{
    match client.head_bucket().bucket(bucket_name).send().await {
        Ok(_) => Ok(true),
        Err(e) => {
            if let Some(code) = e.code() {
                if code == "NotFound" {
                    return Ok(false);
                }
            }
            Err(e.into())
        }
    }
}
async fn get_presigned_url(client: &s3::Client, bucket_name: &str, object_key: &str)->Result<String, failure::Error> {
    let expires_in = Duration::from_secs(120);
    let presigned_request = client.get_object()
                                  .bucket(bucket_name)
                                  .key(object_key)
                                  .presigned(PresigningConfig::expires_in(expires_in)?)
                                  .await?;
    Ok(presigned_request.uri().to_string())
}
pub async fn create_bucket(State(state): State<AppState>,
                           payload: extract::Json<OwnerId>) -> Result<Json<String>, GetFilesError> {
    let client = &state.client;
    client.create_bucket().bucket(&payload.owner_id).send().await
        .map_err(|e| GetFilesError::S3Error(e.into()))?;

    Ok(Json("success".to_string()))

}
pub async fn get_files(State(state): State<AppState>,
                       payload: extract::Json<OwnerId>) -> Result<Json<Vec<FileResponse>>,
                                                                  GetFilesError> {
    let client = &state.client;
    if (check_bucket(&client, &payload.owner_id)).await? {
        println!("Bucked does exist");    
    } else {
      return Err(GetFilesError::NotFound("User bucket not found".to_string()));
    };
    
    // still for fetching from db
    let owner_id = Uuid::parse_str(&payload.owner_id) 
        .map_err(|_| GetFilesError::InternalError("Failed to parse user id".to_string()))?;
    
    /*    let key = thingy.key().unwrap();
        let object_url = get_presigned_url(client, &payload.owner_id, key).await?;
    */
    let pool = &state.pool;
    let files = sqlx::query_as::<_,DatabaseFile>("SELECT * FROM files where owner_id=$1")
        .bind(&owner_id)
        .fetch_all(pool)
        .await
        .map_err(|e| GetFilesError::InternalError(e.to_string()))?;
    let mut response = Vec::with_capacity(files.len());
    for file in files {
        let key = file.file_id.to_string() + "." + &file.extension.clone().expect("REASON");
        let url = match (file.file_type == FileType::Media){
            true => get_presigned_url(client, &payload.owner_id, &key).await?,  
            _ => "".to_string(),
        };
        response.push(FileResponse {
            file_id: file.file_id,
            owner_id: file.owner_id,
            parent_id: file.parent_id,
            file_name: file.file_name,
            extension: file.extension,
            size: file.size,
            file_type: file.file_type,
            created_at: file.created_at,
            last_modified: file.last_modified,
            shared_with: file.shared_with,
            //file: file,
            url:url,
        });
    }
    Ok(Json(response))
}

pub async fn create_folder(State(state): State<AppState>,
                           payload: extract::Json<CreateFolderForm>)->Result<Json<String>, GetFilesError> {
    let user_id = Uuid::parse_str(&payload.owner_id)
        .map_err(|e| GetFilesError::InternalError("Failed to parse user id".to_string()))?;

    let folder_id = Uuid::new_v4();
    let parent_id = match payload.parent_id.is_empty() {
       true => None,
       false => Some(Uuid::parse_str(&payload.parent_id) 
                   .map_err(|_| GetFilesError::InternalError("Failed to parse parent id".to_string()))?),
    };

    let folder_name = payload.folder_name.clone();
    let created_at = Some(Utc::now());
    let last_modified = Some(Utc::now());
    let shared_with: Vec<Uuid> = Vec::new();
    
    let success = match sqlx::query("INSERT into files (file_id, owner_id, parent_id, file_name,
                                     size, file_type, created_at, last_modified, shared_with) 
                                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9);")
        .bind(&folder_id)
        .bind(&user_id)
        .bind(&parent_id)
        .bind(&folder_name)
        .bind(1)
        .bind(FileType::Folder)
        .bind(&created_at)
        .bind(&last_modified)
        .bind(shared_with)
        .execute(&state.pool).await {
            Ok(_) => "Uploaded File",
            Err(e) => return Err(GetFilesError::InternalError(e.to_string())),
        };
    Ok(Json(success.to_string()))
}

//2mb limit 
pub async fn upload_file(State(state): State<AppState>,
                         mut payload: Multipart)->Result<Json<String>, GetFilesError> {
  let mut data = Bytes::new();
  let mut filename = String::new(); 
  let mut content_type = String::new();
  let mut user_id = String::new();
  let mut payload_parent_id = String::new();

  while let Some(field) = payload.next_field().await? {
      match field.name() {
      Some("file") => {
        filename = field.file_name().unwrap_or("unknown").to_string();
        // app/octet - unknown generic type
        content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
        data = field.bytes().await?;
      },
      Some("user_id") => {
        user_id = field.text().await?;
      },
      Some("parent_id") => {
        payload_parent_id = field.text().await?;
      },
      _ => {}
      }
  };
  let client = &state.client;
  if (check_bucket(&client, &user_id)).await? {
    println!("Bucket does exist!");
  } else {
    return Err(GetFilesError::NotFound("User bucket not found".to_string()));
  };

  // ehhh
  let file_size = data.len() as i64; 
  let file_id = Uuid::new_v4();

  let extension = std::path::Path::new(&filename)
      .extension()
      .and_then(|s| s.to_str())
      .unwrap_or("");
  let s3_name = file_id.to_string() + "." + &extension;                        
  client.put_object().bucket(&user_id).key(&s3_name).body(data.into())
      .content_type(&content_type)
      .send()
      .await
      .map_err(|e| GetFilesError::S3Error(e.into()))?;

  let owner_id = Uuid::parse_str(&user_id)
      .map_err(|e| GetFilesError::InternalError(e.to_string()))?;

  let parent_id = match payload_parent_id.is_empty() {
        true => None,
        false => Some(Uuid::parse_str(&payload_parent_id)
            .map_err(|e| GetFilesError::InternalError(e.to_string()))?),
  };
  
  let created_at = Some(Utc::now());
  let last_modified = Some(Utc::now());
  let shared_with: Vec<Uuid> = Vec::new();
  
  let file_type = match content_type.as_str() {
      ctype if ctype.starts_with("image/") => FileType::Media,
      ctype if ctype.starts_with("video/") => FileType::Media,
      ctype if ctype.starts_with("audio/") => FileType::Media,
      ctype if ctype.starts_with("text/") => FileType::Document,
      "application/pdf" => FileType::Document,
      _ => FileType::Other,

  };
  
  let pool = &state.pool;
  sqlx::query("INSERT INTO files (file_id, owner_id, parent_id, file_name,
               size, extension, file_type, created_at, last_modified, shared_with)
               VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10);")
      .bind(&file_id)
      .bind(&owner_id)
      .bind(&parent_id)
      .bind(&filename)
      .bind(file_size)
      .bind(&extension)
      .bind(&file_type)
      .bind(&created_at)
      .bind(&last_modified)
      .bind(shared_with)
      .execute(pool).await
      .map_err(|e| GetFilesError::InternalError(e.to_string()));
                             
  let success = match sqlx::query("UPDATE users
                                   SET storage_used = storage_used + $1
                                   WHERE user_id = $2;")
      .bind(file_size)
      .bind(&owner_id)
      .execute(pool).await {
            Ok(_) => "File uploaded succesfully",
            Err(e) => return Err(GetFilesError::InternalError(e.to_string())),
      };
  Ok(Json(success.to_string()))
}

pub async fn delete_file(State(state): State<AppState>,
                         payload: extract::Json<DeleteFileForm>)->Result<Json<String>, GetFilesError> {
    println!("Ran");
    let file_id = Uuid::parse_str(&payload.file_id) 
        .map_err(|e| GetFilesError::InternalError(e.to_string()))?;
    
    let client = &state.client;
    if (check_bucket(&client, &payload.owner_id)).await? {
        //
    } else {
        return Err(GetFilesError::NotFound("User bucket not found".to_string()));
    };
    let pool = &state.pool;
    let owner_id = Uuid::parse_str(&payload.owner_id)
        .map_err(|e| GetFilesError::InternalError(e.to_string()))?;
    let extension: String = sqlx::query_scalar("DELETE FROM files
                                     WHERE file_id = ($1) AND owner_id = ($2)
                                     RETURNING extension;")//maybe some other way to secure
        .bind(&file_id)
        .bind(&owner_id)
        .fetch_one(pool)
        .await
        .map_err(|e| GetFilesError::InternalError("Database delete failed".to_string()))?;
    // HARD CODED SIZE FOR NOW
    let size = 0;
    sqlx::query("UPDATE users
                 SET storage_used = storage_used - $1
                 WHERE user_id = $2;")
        .bind(&owner_id)
        .bind(&size)
        .execute(pool)
        .await
        .map_err(|e| GetFilesError::InternalError(e.to_string()));  

    let success = match client.delete_object().bucket(&payload.owner_id).key(payload.file_id.clone() + &extension)
        .send().await {
        Ok(_) => format!("Deleted file"),
        Err(e) => return Err(GetFilesError::InternalError(e.to_string())),

    };

    Ok(Json(success.to_string()))
}
