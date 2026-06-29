use crate::models::ServerError;
use uuid::Uuid;
use sqlx::PgPool;


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
fn hash_algorithm(
    password: &str
) -> String {
    "Hello".to_string()
}
