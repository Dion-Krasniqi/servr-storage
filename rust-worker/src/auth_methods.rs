use axum::{extract, extract::State, Json};
use crate::models::{ServerError,
                    AuthState,
                    SignInForm,
                    SignUpForm,
                    Claims};
use jsonwebtoken::{encode, decode, Header, Algorithm, EncodingKey};
use sha2::{Sha256, Digest};
use uuid::Uuid;

fn hash_algorithm(
    password: &str, 
) -> String {
    let hash = Sha256::digest(password);
    format!("{:?}", hash)
}
// acts more like a session token for now
fn create_token(
    data: String,
    expires: usize,
) -> String {
    let claim = Claims { sub: data, exp: expires };
    let token = encode(&Header::default(), &claim, &EncodingKey::from_secret("secret".as_ref()));
    return token
}
pub async fn login_user(
    State(state): State<AuthState>,
    payload: Json<SignInForm>,
) -> Result<(), ServerError> {
    let email = payload.email.trim();
    let user: Option<(String, bool)> = 
        sqlx::query_as(r#"SELECT hashed_password,active from users
                          WHERE email = ($1);"#)
        .bind(&email)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ServerError::DatabaseError(format!("Failed to fetch user. Error: {}", e)))?;
    if user.is_none() {
        println!("user doesnt exist");
        return Err(ServerError::InternalError("User not found".to_string()));
    }
    let hashed_password = if let Some((password, is_active)) = user && is_active {
        password
} else {
        return Err(ServerError::InternalError("User not found or is not active".to_string())); 
    };
    let user_password = hash_algorithm(&payload.password);
    if !(user_password == hashed_password) { 
        return Err(ServerError::InternalError("User password does not match".to_string()));
    }
    Ok(())
}
pub async fn create_user(
    State(state): State<AuthState>,
    payload: Json<SignUpForm>,
) -> Result<(), ServerError> {
    let email = payload.email.trim();
    let user = 
        sqlx::query(r#"SELECT email from users
                          WHERE email = ($1);"#)
        .bind(&email)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ServerError::DatabaseError(format!("Failed to fetch user. Error: {}", e)))?;
    if user.is_some() {
        return Err(ServerError::InternalError("User already exists".to_string()));
    }
    let user_id = Uuid::new_v4(); 
    let hashed_password = hash_algorithm(&payload.password);
    sqlx::query(r#"INSERT INTO users (user_id, 
    email, hashed_password, active, super_user, storage_used)
                   VALUES ($1, $2, $3, $4, $5, $6);"#)
        .bind(&user_id)
        .bind(&email)
        .bind(&hashed_password)
        .bind(true)
        .bind(false)
        .bind(0)
        .execute(&state.pool)
        .await
        .map_err(|e| ServerError::DatabaseError(format!("Failed to create user. Error: {}", e)))?;
    Ok(())

}
