use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Deserialize)]
pub struct OwnerId {
    pub owner_id: String,
}

#[derive(Debug, Serialize, sqlx::Type)]
#[sqlx(type_name="FILETYPE", rename_all="lowercase")]
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
