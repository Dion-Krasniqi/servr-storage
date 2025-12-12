use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use aws_sdk_s3 as s3;
use sqlx::PgPool;
use axum::{response::IntoResponse, http::StatusCode};
//use failure;

#[derive(Deserialize)]
pub struct OwnerId {
    pub owner_id: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
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
    pub size: Option<f32>,
    pub file_type: FileType,
    pub url: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
    pub shared_with: Vec<Uuid>,
}

// uploading
#[derive(Debug,Deserialize)]
pub struct CreateFolderForm {
    pub folder_name: String,
    pub owner_id: String,
    pub parent_id: String,
}

//sharing

//deleting
#[derive(Debug,Deserialize)]
pub struct DeleteFileForm {
    pub owner_id: String, //lowkey not
    pub file_id: String,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub client: s3::Client,
    
}


// error return types
#[derive(Debug)]
pub enum GetFilesError {
    S3Error(s3::Error),
    
    UserIdError,

    FileIdError,

    UserBucketDoesntExist,
    
    InternalError,
}

impl From<s3::Error> for GetFilesError {
    fn from(e: s3::Error) -> Self {
        GetFilesError::S3Error(e)
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
            (_) => (
                    StatusCode::BAD_REQUEST,
                    "General Error".to_string()
                ).into_response(),
        }
    }
}
