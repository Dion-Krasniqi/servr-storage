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

pub async fn get_files(State(state): State<AppState>,
                       payload: extract::Json<OwnerId>) -> Result<Json<Vec<DatabaseFile>>, GetFilesError> {
    
    let client = &state.client;
    if (check_bucket(&client, &payload.owner_id)).await? {
        //
    } else {
      return Err(GetFilesError::NotFound("User bucket not found".to_string()));
    };
    // still for fetching from db
    let owner_id = Uuid::parse_str(&payload.owner_id) 
        .map_err(|_| GetFilesError::InternalError("Failed to get parse user id".to_string()))?;
    
    let list_objects_output = match client.list_objects_v2().bucket(&payload.owner_id).send().await {
        Ok(res) => res,
        Err(e) => return Err(GetFilesError::InternalError("Failed to get user objects".to_string())),
    };
    for thingy in list_objects_output.contents() {
        let key = thingy.key().unwrap();
        let object_url = get_presigned_url(client, &payload.owner_id, key).await?;
        println!("Url: {}", object_url);
    };

    let pool = &state.pool;
    let files = match sqlx::query_as::<_,DatabaseFile>("SELECT * FROM files where owner_id=$1")
        .bind(&owner_id)
        .fetch_all(pool)
        .await {
            Ok(f) => f,
            Err(e) => return Err(GetFilesError::InternalError(/*"Failed to fetch files from database"*/e.to_string())),
    };
    for file in &files {
        println!("{}",file.file_id);
    };
    Ok(Json(files))
}

pub async fn create_folder(pool: extract::State<PgPool>,
                           payload: extract::Json<CreateFolderForm>)->Result<Json<String>, String> {
    let user_id = match Uuid::parse_str(&payload.owner_id) {
        Ok(id) => id,
        Err(e) => return Err(format!("Failed to get user id: {}", e)),
    };

    let folder_id = Uuid::new_v4();
    let parent_id = match payload.parent_id.is_empty() {
       true => None,
       false => match Uuid::parse_str(&payload.parent_id) {
            Ok(id) => Some(id),
            Err(e) => return Err(format!("Failed to get parent id: {}", e)),
        }
    };

    let folder_name = payload.folder_name.clone();
    let created_at = Some(Utc::now());
    let last_modified = Some(Utc::now());
    let shared_with: Vec<Uuid> = Vec::new();
    println!("{}",folder_name);
    println!("{}",folder_id);
    println!("{}", user_id);
    let success = match sqlx::query("INSERT into files (file_id, owner_id, parent_id, file_name,
                                       size, file_type, created_at, last_modified, shared_with) 
                                       VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9);")
        .bind(&folder_id)
        .bind(&user_id)
        .bind(&parent_id)
        .bind(&folder_name)
        .bind(0_f32)
        .bind(FileType::Folder)
        .bind(&created_at)
        .bind(&last_modified)
        .bind(shared_with)
        .execute(&pool.0).await {
            Ok(_) => "Folder Created",
            Err(e) => return Err(format!("Failed to created folder: {}", e)),
    };

    Ok(Json(success.to_string()))
}

//2mb limit 
pub async fn upload_file(pool: extract::State<PgPool>,
                         mut payload: Multipart)->Result<Json<String>, String> {
  let mut data: Bytes;
  let mut user_id = String::new();
  let mut payload_parent_id = String::new();
  while let Some(field) = payload.next_field().await.unwrap() {
      let name = field.name().unwrap().to_string();
      if name == "file" {
        data = field.bytes().await.unwrap();
        //
      } else if name == "user_id" {
        user_id = field.text().await.unwrap().to_string();
        // let owner_id = match Uuid:parse_str...
      } else if name == "parent_id" {
        payload_parent_id = field.text().await.unwrap().to_string();
        // same stuff
      }
  };
  let owner_id = match Uuid::parse_str(&user_id) {
    Ok(id) => Some(id),
    Err(e) => return Err(format!("Failed to parse user id: {}", e)),
  };

  let parent_id = match payload_parent_id.is_empty() {
        true => None,
        false => match Uuid::parse_str(&payload_parent_id) {
            Ok(id) => Some(id),
            Err(e) => return Err(format!("Failed to parse parent id: {}", e)),
        }
  };

  let file_id = Uuid::new_v4();
  let file_size = 1;
  let file_name = "Name";
  let created_at = Some(Utc::now());
  let last_modified = Some(Utc::now());
  let shared_with: Vec<Uuid> = Vec::new();
  let extension = ".png";
  let file_type = "Type";
  let success = match sqlx::query("INSERT INTO files (file_id, owner_id, parent_id, file_name,
                                   size, extension, file_type, created_at, last_modified, shared_with)
                                   VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10);")
      .bind(&file_id)
      .bind(&owner_id)
      .bind(&parent_id)
      .bind(&file_name)
      .bind(&file_size)
      .bind(&extension)
      .bind(&file_type)
      .bind(&created_at)
      .bind(&last_modified)
      .bind(shared_with)
      .execute(&pool.0).await {
            Ok(_) => "File Uploaded",
            Err(e) => return Err(format!("Failed to upload file {}. Error: {}", &file_name, e)),
      };

  Ok(Json(success.to_string()))
}

pub async fn delete_file(pool: extract::State<PgPool>,
                         payload: extract::Json<DeleteFileForm>)->Result<Json<String>,String> {

    let owner_id = match Uuid::parse_str(&payload.owner_id) {
        Ok(id) => id,
        Err(e) => return Err(format!("Failed to parse owner id: {}", e))
    };

    let file_id = match Uuid::parse_str(&payload.file_id) {
        Ok(id) => id,
        Err(e) => return Err(format!("Failed to parse file id: {}", e))
    };

    let success = match sqlx::query("DELETE FROM files
                                     WHERE file_id = ($1) AND owner_id = ($2);")//maybe some other way to secure
        .bind(&file_id)
        .bind(&owner_id)
        .execute(&pool.0).await {
            Ok(_) => "File Deleted",
            Err(e) => return Err(format!("Failed to delete folder: {}", e)),
        };

    Ok(Json(success.to_string()))
}
