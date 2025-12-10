use axum::{extract, Json};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;

use crate::models::{DatabaseFile, OwnerId, CreateFolderForm, CreateFileForm, FileType};

pub async fn get_files(pool: extract::State<PgPool>,
                       payload: extract::Json<OwnerId>)->Result<Json<Vec<DatabaseFile>>, String> {
    let owner_id = match Uuid::parse_str(&payload.owner_id) {
        Ok(id) => id,
        Err(e) => return Err(format!("Failed to get owner id: {}", e)),
    };

    let files = match sqlx::query_as::<_,DatabaseFile>("SELECT * FROM files where owner_id=$1")
        .bind(&owner_id)
        .fetch_all(&pool.0)
        .await {
            Ok(f) => f,
            Err(e) => return Err(format!("Database error: {}", e)),
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

pub async fn upload_file(pool: extract::State<PgPool>,
                        payload: extract::Json<CreateFileForm>)->Result<Json<String>, String> {
  let user_id = match Uuid::parse_str(&payload.owner_id) {
        Ok(id) => id,
        Err(e) => return Err(format!("Failed to get user id: {}", e)),
  };

  let parent_id = match (payload.parent_id.is_empty()) {
        true => None,
        false => match Uuid::parse_str(&payload.parent_id) {
            Ok(id) => Some(id),
            Err(e) => return Err(format!("Failed to get parent id: {}", e)),

        }
  };

  let file_id = Uuid::new_v4();

  let file_size = match (*&payload.size >= 0.0) {
    true => payload.size.clone(),
    false => 0.0,
  };
  let file_name = payload.file_name.clone();
  let created_at = Some(Utc::now());
  let last_modified = Some(Utc::now());
  let shared_with: Vec<Uuid> = Vec::new();

  let success = match sqlx::query("INSERT INTO files (file_id, owner_id, parent_id, file_name,
                                   size, extension, file_type, created_at, last_modified, shared_with)
                                   VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10);")
      .bind(&file_id)
      .bind(&user_id)
      .bind(&parent_id)
      .bind(&file_name)
      .bind(&file_size)
      .bind(&payload.extension)
      .bind(&payload.file_type)
      .bind(&created_at)
      .bind(&last_modified)
      .bind(shared_with)
      .execute(&pool.0).await {
            Ok(_) => "File Uploaded",
            Err(e) => return Err(format!("Failed to upload file {}. Error: {}", &file_name, e)),
      };

  Ok(Json(success.to_string()))
}
