use crate::models::ServerError;
use uuid::Uuid;
use sha2::{Sha256, Digest};
use sqlx::PgPool;
use aws_sdk_s3 as s3;

pub async fn get_user_id(
    email: &str,
    password: &str,
    pool: &PgPool,
) -> Result<Uuid, ServerError> {
    let user: Option<(Uuid, String, bool)> = 
        sqlx::query_as(r#"SELECT user_id,hashed_password,active from users
                          WHERE email = ($1);"#)
        .bind(&email)
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerError::DatabaseError(format!("Failed to fetch user. Error: {}", e)))?;
    if user.is_none() {
        println!("user doesnt exist");
        return Err(ServerError::InternalError("User not found".to_string()));
    }
    let (hashed_password, user_id) = if let Some((id, password, is_active)) = user && is_active {
        (password, id)
    } else {
        return Err(ServerError::InternalError("User not found or is not active".to_string())); 
    };
    let user_password = hash_algorithm(password);
    if !(user_password == hashed_password) { 
        return Err(ServerError::InternalError("User password does not match".to_string()));
    }
    Ok(user_id)
}
pub fn hash_algorithm(
password: &str, 
) -> String {
    let hash = sha2::Sha256::digest(password);
    //format!("{:x}", hash)
    hash.iter().map(|a| format!("{:02x}", a)).collect()
}

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


