use axum::{extract, Json};
use sqlx::PgPool;
use uuid::Uuid;
use crate::models::{DatabaseFile, OwnerId};

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


