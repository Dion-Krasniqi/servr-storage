use axum::{extract, extract::State, Json, http::StatusCode};
use axum_extra::extract::Multipart;
use aws_sdk_s3 as s3;
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::presigning::PresigningConfig;
use sqlx::Acquire;
use axum_extra::extract::cookie::{Cookie, CookieJar};
use jsonwebtoken::{encode, decode, Header, Algorithm, EncodingKey,
                   DecodingKey, Validation};

use uuid::Uuid;
use chrono::{DateTime, Utc};
use bytes::Bytes;
use std::time::Duration;
use std::path::Path;
use std::collections::HashMap;
use std::sync::Arc;
use failure;
use serde_json::Value;

use crate::models::{DatabaseFile,
                    FileResponse,
                    OwnerId, 
                    CreateFolderForm, 
                    FileType, 
                    DeleteFileForm,
                    RenameFileForm,
                    DownloadFileForm,
                    AppState,
                    ServerError};
use crate::auth_methods::get_current_user;

async fn check_bucket(client: &s3::Client, bucket_name: &str)->Result<bool, s3::Error>{
    match client.head_bucket().bucket(bucket_name).send().await {
        Ok(_) => Ok(true),
        Err(e) => {
            if let Some(code) = e.code() {
                if code == "NotFound" || code == "NoSuchBucket" {
                    return Ok(false);
                }
            }
            eprintln!("Error {:?}", e);
            Err(e.into())
        }
    }
}

async fn get_presigned_url(client: &s3::Client, bucket_name: &str, object_key: &str)->Result<String, failure::Error> {
    let expires_in = Duration::from_secs(604800);  //7days
    match client.get_object()
                            .bucket(bucket_name)
                            .key(object_key)
                            .presigned(PresigningConfig::expires_in(expires_in)?)
                            .await {
                                    Ok(link) => Ok(link.uri().to_string()),
                                    Err(e) => {
                                                eprintln!("Error {:?}", e);
                                                return Err(e.into())
                                              }
                                  }
}

fn s3_key(file_id: String, file_ext: &Option<String>)->String{
    if let Some(e) = file_ext && e != "" {
        return file_id + "." + e;
    }
    return file_id;
}
fn update_url(//file: &FileResponse, 
              file_url: &Option<String>,
              file_modified: &Option<DateTime<Utc>>,
              cur_date: DateTime<Utc>
) -> bool {
    file_url.as_ref().map_or(true, |u| u.is_empty())
            || file_modified.is_none() 
            || file_modified.as_ref()
        .map(|date| *date + chrono::Duration::days(6) < cur_date)
        .unwrap_or(true)
}

pub async fn create_bucket(State(state): State<AppState>,
                           payload: extract::Json<OwnerId>
) -> Result<Json<String>, ServerError> {
    match state.client.create_bucket()
        .bucket(&payload.owner_id)
        .send()
        .await{   
            Ok(_) => Ok(Json("Success".to_string())),
            Err(e) => {
                        eprintln!("Error {:?}", e);    
                        return Err(ServerError::S3Error(e.into()))
            }
    }
}
// same thing but above serves as endpoint currently
pub async fn create_bucket_func(client: s3::Client,
                        owner_id: &str,
) -> Result<(), ServerError> {
    match client.create_bucket()
        .bucket(owner_id)
        .send()
        .await{   
            Ok(_) => return Ok(()),
            Err(e) => {
                        eprintln!("Error {:?}", e);    
                        return Err(ServerError::S3Error(e.into()))
            }
    }
}

pub async fn get_files(State(state): State<AppState>,
                       jar: CookieJar,
)-> Result<Json<HashMap<Uuid, FileResponse>>, ServerError> {
    
    println!("We are fetching!");
    let user_id = if let Ok(id) = get_current_user(jar, &state.key, &state.cache).await 
    && id != "NOT VALID" {
        id
    } else {
        return Err(ServerError::Unauthorized("No session token found".to_string()));
    };
    let owner_id = Uuid::parse_str(&user_id)
            .map_err(|_| ServerError::InternalError("Failed to parse user id".to_string()))?;
    let client = &state.client;
    let pool = &state.pool;
 
    let cur_date = Utc::now();
    // user bucket checked after this, think abt it
    if let Some(c) = state.cache.get(&owner_id).await { 
        println!("Cache hit, {} items", c.len()); 
        let to_update: Vec<Uuid> = c
            .iter()
            .filter(|(_,file)| update_url(&file.url, &file.last_modified, cur_date))
            .map(|(id,_)| *id ).collect();
        if to_update.is_empty() {
            return Ok(Json((*c).clone()));
        }
        let mut e: HashMap<Uuid, FileResponse> = (*c).clone();
        let mut updated_urls: Vec<String> = Vec::new();
        for file_id in &to_update {
            let key = s3_key(file_id.to_string(), &(e[&file_id].extension));
            let file_url = get_presigned_url(client, &user_id, &key).await?;
            e.entry(*file_id).and_modify(|f| { 
                                f.url = Some(file_url.clone());                            
                                f.last_modified = Some(cur_date);
            });
            updated_urls.push(file_url);
        }
        sqlx::query(r#"UPDATE files SET last_modified = ($1),
                                                            url = updated.url
                                                        FROM UNNEST($2::varchar[],
                                                                    $3::uuid[])
                                                        AS updated(url, id)
                                                        WHERE file_id = updated.id;"#) 
                                        .bind(&cur_date)
                                        .bind(&updated_urls)
                                        .bind(&to_update)
                                        .execute(&state.pool)
                                        .await
                                        .map_err(|e| 
                                            ServerError::DatabaseError(e.to_string()))?;            
        state.cache.remove(&owner_id).await;
        state.cache.insert(owner_id, Arc::new(e.clone())).await;
        return Ok(Json(e));
    }

    if !((check_bucket(&client, &user_id)).await?) {
        println!("User bucket not found!");
        return Err(ServerError::NotFound("User bucket not found".to_string()));
    }
 
    let files = sqlx::query_as::<_,DatabaseFile>("SELECT * FROM files where owner_id = ($1);")
        .bind(&owner_id)
        .fetch_all(pool)
        .await
        .map_err(|e| {  eprintln!("{:?}", e);
                        ServerError::DatabaseError(e.to_string())})?;
    if files.len() == 0 {
        // idk
        return Ok(Json(HashMap::new()));
    }
    let mut file_map: HashMap<Uuid, FileResponse> = HashMap::new();
    let mut to_update_ids: Vec<Uuid> = Vec::new();
    let mut to_update_urls: Vec<String> = Vec::new();
    for mut file in files {
        if update_url(&file.url, &file.last_modified, cur_date) {
            let key = s3_key(file.file_id.to_string(), &file.extension);
            let file_url = get_presigned_url(client, &user_id, &key).await?;
            file.url = Some(file_url.clone());
            to_update_ids.push(file.file_id);            
            to_update_urls.push(file_url);
        }
        file_map.insert(file.file_id,
            FileResponse {
            file_id: file.file_id,
            owner_id: file.owner_id,
            parent_id: file.parent_id,
            file_name: file.file_name,
            extension: file.extension,
            size: file.size,
            file_type: file.file_type,
            created_at: file.created_at,
            last_modified: Some(cur_date),
            shared_with: file.shared_with,
            url: file.url,
        });
    }
    sqlx::query(r#"UPDATE files SET last_modified = ($1),
                   url = updated.url
                   FROM UNNEST($2::varchar[],$3::uuid[]) AS updated(url, id)
                   WHERE file_id = updated.id;"#) 
                                        .bind(&cur_date)
                                        .bind(&to_update_urls)
                                        .bind(&to_update_ids)
                                        .execute(&state.pool)
                                        .await
                                        .map_err(|e| 
                                            ServerError::DatabaseError(e.to_string()))?;            


    state.cache.insert(owner_id, Arc::new(file_map.clone())).await;
    Ok(Json(file_map))
}


pub async fn create_folder(State(state): State<AppState>,
                           jar: CookieJar,
                           payload: Json<CreateFolderForm>
)->Result<StatusCode, ServerError> {

    println!("CreateFolder ran");
    let owner_id = if let Ok(id) = get_current_user(jar, &state.key, &state.cache).await 
    && id != "NOT VALID" {
        Uuid::parse_str(&id)
            .map_err(|_| ServerError::InternalError("Failed to parse user id".to_string()))?
    } else {
        return Err(ServerError::Unauthorized("No session token found".to_string()));
    };
    let folder_id = Uuid::new_v4();
    
    let parent_id = match payload.parent_id.is_empty() {
       true => None,
       false => Some(Uuid::parse_str(&payload.parent_id) 
                   .map_err(|e| ServerError::InternalError(format!("Failed to parse parent id. Error: {}", e)))?),
    };

    let folder_name =  payload.folder_name.trim();
    if folder_name.is_empty() {
        println!("Name empty");
        return Err(ServerError::InternalError("Invalid name".to_string()));
    };

    let created_at = Some(Utc::now());
    let shared_with: Vec<Uuid> = Vec::new();
    
    match sqlx::query(r#"INSERT into files (file_id, owner_id, parent_id, file_name,
                       size, file_type, created_at, last_modified, shared_with) 
                       VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9);"#)
        .bind(&folder_id)
        .bind(&owner_id)
        .bind(&parent_id)
        .bind(&folder_name)
        .bind(0)
        .bind(FileType::Folder)
        .bind(&created_at)
        .bind(&created_at)
        .bind(&shared_with)
        .execute(&state.pool).await {
            Ok(_) => {},
            Err(e) => {
                        eprintln!("Error {:?}", e);
                        return Err(ServerError::DatabaseError(e.to_string()))
            },
    }
    let new_folder = FileResponse {
        file_id: folder_id,
        owner_id: owner_id,
        parent_id: parent_id,
        file_name: folder_name.to_string(),
        extension: None,
        size: 0,
        file_type: FileType::Folder,
        created_at: created_at,
        last_modified: created_at,
        shared_with: shared_with,
        url: None,
    };

    let files = if let Some(c) = state.cache.get(&owner_id).await {
            let mut cached_files = (*c).clone();
            cached_files.insert(folder_id, new_folder.clone());
            cached_files
    } else {
            let new_files: HashMap<Uuid, FileResponse> = HashMap::from(
                [(folder_id, new_folder.clone()),]);
            new_files
    };
    state.cache.insert(owner_id, Arc::new(files)).await;

    Ok(StatusCode::CREATED)
}

//2mb limit 
pub async fn upload_file(State(state): State<AppState>,
                         jar: CookieJar,
                         mut payload: Multipart,
)->Result<Json<String>, ServerError> {

  println!("UploadFile Ran");
  
  let user_id = if let Ok(id) = get_current_user(jar, &state.key, &state.cache).await 
  && id != "NOT VALID" {
      id
  } else {
        return Err(ServerError::Unauthorized("No session token found".to_string()));
  };
  if check_bucket(&state.client, &user_id).await? {
      println!("Bucket does exit");
  } else {
    println!("User bucket not found");
    return Err(ServerError::NotFound("User bucket not found".to_string()));
  };
  let mut data = Bytes::new();
  let mut filename = String::new(); 
  let mut content_type = String::new();
  let mut payload_parent_id = String::new();

  while let Some(field) = payload.next_field().await? {
      match field.name() {
      Some("file") => {
        filename = field.file_name().unwrap_or("unknown").to_string();
        // app/octet - unknown generic type
        content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
        data = field.bytes().await?;
      },
      // see about this one since move getting id frm cookies
      Some("user_id") => {
        //user_id = field.text().await?;
      },
      Some("parent_id") => {
        payload_parent_id = field.text().await?;
      },
      _ => {}
      }
  };
  

  let file_size = data.len() as i64; 
  let owner_id = Uuid::parse_str(&user_id)
      .map_err(|e| ServerError::InternalError(e.to_string()))?;

  let storage_used: i64 = sqlx::query_scalar(r#"SELECT storage_used
                                                FROM users
                                                WHERE user_id = ($1);"#)
      .bind(&owner_id)
      .fetch_one(&state.pool)
      .await
      .map_err(|e| ServerError::DatabaseError(format!("Failed to get storage. Error: {}", e)))?;

  if (file_size + storage_used) > 1048576 * 2 {
        return Err(ServerError::InternalError("Not enough storage".to_string())); 
  }

  let file_id = Uuid::new_v4();  
  let parent_id = match payload_parent_id.is_empty() {
        true => None,
        false => Some(Uuid::parse_str(&payload_parent_id)
            .map_err(|e| ServerError::InternalError(e.to_string()))?),
  };

  let name = Path::new(&filename).file_stem()
      .and_then(|s| s.to_str()).unwrap_or("unknown");
  let extension = Path::new(&filename)
      .extension()
      .and_then(|s| s.to_str())
      .unwrap_or("");
  
  let created_at = Some(Utc::now());
  let shared_with: Vec<Uuid> = Vec::new();
  let file_type = match content_type.as_str() {
      ctype if ctype.starts_with("image/") => FileType::Media,
      ctype if ctype.starts_with("video/") => FileType::Media,
      ctype if ctype.starts_with("audio/") => FileType::Media,
      ctype if ctype.starts_with("text/") => FileType::Document,
      "application/pdf" => FileType::Document,
      _ => FileType::Other,

  };
 let mut conn = state.pool.acquire().await.map_err(|e| ServerError::DatabaseError(e.to_string()))?;
 let mut tx = conn.begin().await.map_err(|e| ServerError::DatabaseError(e.to_string()))?;
 
 // user table update
 match sqlx::query(r#"UPDATE users
              SET storage_used = storage_used + ($1)
              WHERE user_id = ($2);"#)
              .bind(file_size)
              .bind(&owner_id)
              .execute(&mut *tx)
              .await {
                   Ok(_) => println!("User Table Update"),
                   Err(e) => {
                              eprintln!("Error {:?}", e);
                              return Err(ServerError::DatabaseError(e.to_string()))
                   }
              }
 // file table update
 match sqlx::query(r#"INSERT INTO files (file_id, owner_id, parent_id, file_name,
              size, extension, file_type, created_at, last_modified, shared_with)
              VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10);"#)
      .bind(&file_id)
      .bind(&owner_id)
      .bind(&parent_id)
      .bind(name)
      .bind(file_size)
      .bind(&extension)
      .bind(&file_type)
      .bind(&created_at)
      .bind(&created_at)
      .bind(&shared_with)
      .execute(&mut *tx)
      .await {
                   Ok(_) => println!("File Table Update"),
                   Err(e) => {
                              eprintln!("Error {:?}", e);
                              return Err(ServerError::DatabaseError(e.to_string()))
                   }
      }
  if let Some(parent_id) = parent_id {
        match sqlx::query(r#"WITH RECURSIVE ancestors AS (
                                                    SELECT file_id, parent_id
                                                    FROM files
                                                    WHERE file_id = ($1)
                                                    UNION ALL

                                                    SELECT f.file_id, f.parent_id
                                                    FROM files f
                                                    JOIN ancestors a ON f.file_id = a.parent_id
                                                 )
                     UPDATE FILES
                     SET size = size + ($2)
                     WHERE file_id IN (SELECT file_id FROM ancestors);"#)
            .bind(&parent_id)
            .bind(&file_size)
            .execute(&mut *tx)
            .await {
                   Ok(_) => println!("Parent Update"),
                   Err(e) => {
                              eprintln!("Error {:?}", e);
                              return Err(ServerError::DatabaseError(e.to_string()))
                            }
            }
  } 
  let s3_name = s3_key(file_id.to_string(),&Some(extension.to_string())); 
  match state.client.put_object().bucket(&user_id).key(&s3_name).body(data.into())
          .content_type(&content_type)
          .send()
          .await { 
                   Ok(_) => {},
                   Err(e) => {
                              eprintln!("Error {:?}", e);
                              return Err(ServerError::S3Error(e.into()))
                    },
  }
  match tx.commit()
      .await {
            Ok(_) => {},
            Err(e) => {
                        eprintln!("Error {:?}", e);
                        return Err(ServerError::DatabaseError(e.to_string()))
                      }
  }
  let uploaded_file = FileResponse {
    file_id: file_id,
    owner_id: owner_id,
    parent_id: parent_id,
    file_name: name.to_string(),
    extension: Some(extension.to_string()),
    size: file_size,
    file_type:FileType::Media,
    created_at: created_at,
    last_modified: created_at,
    shared_with: shared_with.clone(),
    url: None,
  };
 
  let cached_files: HashMap<Uuid, FileResponse> = if let Some(c) = state.cache
  .get(&owner_id).await {
          let mut e = (*c).clone();
          e.insert(file_id, uploaded_file);
          e
  } else { 
      HashMap::from([(file_id, uploaded_file)],) 
  };
  state.cache.insert(owner_id, Arc::new(cached_files)).await;
  Ok(Json("File Uploaded".to_string()))
}


pub async fn delete_file(State(state): State<AppState>,
                         jar: CookieJar,
                         payload: extract::Json<DeleteFileForm>
)->Result<Json<String>, ServerError> {

    println!("DeleteFile Ran");
    if let Ok(id) = get_current_user(jar, &state.key, &state.cache).await {
      if !(payload.owner_id == id) {
          return Err(ServerError::Unauthorized("Unauthorized".to_string()));
      }
    } else {
        return Err(ServerError::Unauthorized("No session token found".to_string()));
    };

    if !((check_bucket(&state.client, &payload.owner_id)).await?) {
        println!("User bucket not found!");
        return Err(ServerError::NotFound("User bucket not found".to_string()));
    };
    let owner_id = Uuid::parse_str(&payload.owner_id)
        .map_err(|e| ServerError::InternalError(e.to_string()))?;


    let file_id = Uuid::parse_str(&payload.file_id) 
        .map_err(|e| ServerError::InternalError(e.to_string()))?; 
    let mut conn = state.pool.acquire().await.map_err(|e| ServerError::DatabaseError(e.to_string()))?;
    let mut tx = conn.begin().await.map_err(|e| ServerError::DatabaseError(e.to_string()))?;
    
    // delete file from db
    let (extension, size, parent_id): (Option<String>, i64, Option<Uuid>) = sqlx::query_as(r#"DELETE FROM files
                                     WHERE file_id = ($1) AND owner_id = ($2)
                                     RETURNING extension, size, parent_id;"#)
        .bind(&file_id)
        .bind(&owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ServerError::DatabaseError(format!("Database delete failed. Error: {}", e)))?;
    // update user storage
    sqlx::query(r#"UPDATE users
                 SET storage_used = storage_used - ($1)
                 WHERE user_id = ($2);"#)
        .bind(size)
        .bind(&owner_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerError::DatabaseError(e.to_string()))?;  
    // reduce respective storage from parent folders if any
    if let Some(parent_id) = parent_id {
        sqlx::query(r#"WITH RECURSIVE ancestors AS (
                                                    SELECT file_id, parent_id
                                                    FROM files
                                                    WHERE file_id = ($1)
                                                    UNION ALL

                                                    SELECT f.file_id, f.parent_id
                                                    FROM files f
                                                    JOIN ancestors a ON f.file_id = a.parent_id
                                                 )
                     UPDATE FILES
                     SET size = size - ($2)
                     WHERE file_id IN (SELECT file_id FROM ancestors);"#)
            .bind(parent_id)
            .bind(size)
            .execute(&mut *tx)
            .await.map_err(|e| ServerError::DatabaseError(e.to_string()))?;
    }

    let ext = extension.clone().unwrap_or("".to_string());
    let key = s3_key(payload.file_id.to_string(), &Some(ext));//payload.file_id.to_string() + "." + &ext;

    match state.client.delete_object().bucket(&payload.owner_id).key(key)
        .send().await {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Error {:?}", e);
                    return Err(ServerError::S3Error(e.into()))
                },
    }
    match tx.commit().await {
                Ok(_) => {},
                Err(e) => {
                            eprintln!("Error {:?}", e);
                            return Err(ServerError::DatabaseError(e.to_string()))
                },
    }
    if let Some(c) = state.cache.get(&owner_id).await {
        let mut e = (*c).clone();
        e.remove(&file_id);
        state.cache.remove(&owner_id).await;
        state.cache.insert(owner_id, Arc::new(e)).await;
    }
                                            
    Ok(Json("File Deleted".to_string()))
}

pub async fn rename_file(State(state): State<AppState>, 
                         jar: CookieJar,                
                         payload: Json<RenameFileForm>,
)->Result<Json<String>, ServerError> {

    println!("Rename ran");
    if let Ok(id) = get_current_user(jar, &state.key, &state.cache).await {
      if !(payload.owner_id == id) {
          return Err(ServerError::Unauthorized("Unauthorized".to_string()));
      }
    } else {
        return Err(ServerError::Unauthorized("No session token found".to_string()));
    };
    let owner_id = Uuid::parse_str(&payload.owner_id)
        .map_err(|e| ServerError::InternalError(e.to_string()))?;

    let name =  payload.file_name.trim();
    if name.is_empty() {
        println!("Name empty");
        return Err(ServerError::InternalError("Invalid name".to_string()));
    };
    let file_id = Uuid::parse_str(&payload.file_id)
        .map_err(|e| ServerError::InternalError(e.to_string()))?;    
    match sqlx::query(r#"UPDATE files
                         SET file_name = ($1)
                         WHERE file_id = ($2) AND owner_id = ($3);"#)
        .bind(&name)
        .bind(&file_id)
        .bind(&owner_id)
        .execute(&state.pool)
        .await {
            Ok(_) => {},
            Err(e) => {
                        eprintln!("Error {:?}", e);
                        return Err(ServerError::DatabaseError(e.to_string()))
            },
        }
    if let Some(c) = state.cache.get(&owner_id).await {
            let mut e = (*c).clone();
            e.entry(file_id)
                .and_modify(|f| f.file_name = name.to_string());
            state.cache.remove(&owner_id).await;
            state.cache.insert(owner_id, Arc::new(e)).await;
    }
   
    Ok(Json("File Renamed".to_string()))
}
pub async fn download_file(State(state): State<AppState>,
                           jar: CookieJar,
                           payload: extract::Json<DownloadFileForm>
) -> Result<Json<serde_json::Value>, ServerError> {
    
    if let Ok(id) = get_current_user(jar, &state.key, &state.cache).await {
      if !(payload.owner_id == id) {
          return Err(ServerError::Unauthorized("Unauthorized".to_string()));
      }
    } else {
        return Err(ServerError::Unauthorized("No session token found".to_string()));
    };
    // this should allow for when email sharing so owner_id and requester id can be diff
    let owner_id = Uuid::parse_str(&payload.owner_id)
        .map_err(|e| ServerError::InternalError(e.to_string()))?;
    let file_id = Uuid::parse_str(&payload.file_id)
        .map_err(|e| ServerError::InternalError(e.to_string()))?;

    let cur_date = Utc::now();
    let mut file_name = payload.file_id.clone();
    let mut url: Option<String> = None;
    if let Some(c) = state.cache.get(&owner_id).await {
        if let Some(cached_file) = (*c).clone().get(&file_id) {
            if let Some(c_url) = &cached_file.url {
                if let Some(date) = cached_file.last_modified {
                   if date + chrono::Duration::days(6) >= cur_date {
                        file_name = cached_file.file_name.clone();
                        url = Some(c_url.to_string());
                   }
                }
            }
        }
    }
    if url.is_none() {
        let row = sqlx::query_as::
        <_,(Option<String>,
            String,
            Option<DateTime<Utc>>)>
            (r#"SELECT url, file_name, last_modified FROM files
                                      WHERE file_id = ($1) 
                                      AND owner_id = ($2);"#) 
                .bind(&file_id)
                .bind(&owner_id)
                .fetch_optional(&state.pool)
                .await
                .map_err(|e| ServerError::DatabaseError(e.to_string()))?;
        match row {
            Some((Some(database_url),fetched_file_name, Some(date)))
                if date + chrono::Duration::days(6) >= cur_date => {
                // updating cache 
                if let Some(c) = state.cache.get(&owner_id).await {
                    let mut e = (*c).clone();
                    e.entry(file_id)
                        .and_modify(|f| {
                            f.url = Some(database_url.clone());
                        });
                    state.cache.remove(&owner_id).await;
                    state.cache.insert(owner_id.clone(), Arc::new(e)).await;
                };
                url = Some(database_url);
                file_name = fetched_file_name;
            },
            _ => {  
                    let extension =  payload.file_extension.clone().unwrap_or("".to_string());
                    let key = s3_key(payload.file_id.to_string(), &Some(extension));
                    let new_url = get_presigned_url(&state.client,
                                    &payload.owner_id, &key).await?;

                    let fetched_file_name = sqlx::query_as::<_,(String,)>(r#"UPDATE files
                                   SET last_modified = ($1),
                                   url = ($2)
                                   WHERE file_id = ($3)
                                   RETURNING file_name;"#) 
                    .bind(&cur_date)
                    .bind(&new_url)
                    .bind(&file_id)
                    .fetch_one(&state.pool)
                    .await.map_err(|e| ServerError::DatabaseError(e.to_string()))?; 

                   url = Some(new_url);
                   file_name = fetched_file_name.0;
            },
        }
    };
    let url = url
        .ok_or(ServerError::InternalError("Failed to get URL"
        .to_string()))?;
    println!("{}", url);
    let v: Value = serde_json::json!({"url":url, "file_name":file_name});
    Ok(Json(v))
}
