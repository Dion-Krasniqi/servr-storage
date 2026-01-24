use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use aws_sdk_s3 as s3;
use sqlx::PgPool;
use axum::{response::IntoResponse, http::StatusCode};
use axum_extra::extract::multipart::MultipartError;
use failure;
use moka::sync::Cache;

#[derive(Deserialize)]
pub struct OwnerId {
    pub owner_id: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type,PartialEq)]
#[sqlx(type_name="FILETYPE", rename_all="lowercase")]
#[serde(rename_all = "lowercase")] //for deserializing
pub enum FileType { Media, Document, Other, Folder }

//add deserialize aswell
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DatabaseFile {
    pub file_id: Uuid,
    pub owner_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub file_name: String,
    pub extension: Option<String>,
    pub size: i64,
    pub file_type: FileType,
    pub created_at: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
    pub url: Option<String>,
    pub shared_with: Vec<Uuid>,
}
//same thing for now
#[derive(Debug, Serialize)]
pub struct FileResponse {
    //pub file: DatabaseFile
    pub file_id: Uuid,
    pub owner_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub file_name: String,
    pub extension: Option<String>,
    pub size: i64,
    pub file_type: FileType,
    pub created_at: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
    pub shared_with: Vec<Uuid>,
    pub url: String,
}

// uploading
#[derive(Debug,Deserialize)]
pub struct CreateFolderForm {
    pub owner_id: String,
    pub folder_name: String,
    pub parent_id: String,
}

//sharing

//deleting
#[derive(Debug,Deserialize)]
pub struct DeleteFileForm {
    pub owner_id: String,
    pub file_id: String,
}
//renaming
#[derive(Debug,Deserialize)]
pub struct RenameFileForm {
    pub owner_id: String,
    pub file_id: String,
    pub file_name: String,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub client: s3::Client,
    pub cache: Cache<String,String>,// make the 2nd string the actual val obv
    
}


// error return types
#[derive(Debug)]
pub enum GetFilesError {
    S3Error(s3::Error),
    // maybe ref
    InternalError(String),
    NotFound(String),    

    
}

impl From<s3::Error> for GetFilesError {
    fn from(e: s3::Error) -> Self {
        GetFilesError::S3Error(e)
    }
}
impl From<failure::Error> for GetFilesError {
    fn from(e: failure::Error) -> Self {
        GetFilesError::InternalError(e.to_string())
    }

}
impl From<MultipartError> for GetFilesError {
    fn from(e: MultipartError) -> Self {
        GetFilesError::InternalError(e.to_string())
    }
}
//for axum
impl IntoResponse for GetFilesError {
    fn into_response(self) -> axum::response::Response {
        match self {
            GetFilesError::S3Error(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "S3 error".to_string(),
                ).into_response(),
                //split this bcuz currently too broad
            GetFilesError::InternalError(msg) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    msg,
                ).into_response(),
            GetFilesError::NotFound(msg) => (
                    StatusCode::NOT_FOUND,
                    msg,
                ).into_response(),
        }
    }
}
