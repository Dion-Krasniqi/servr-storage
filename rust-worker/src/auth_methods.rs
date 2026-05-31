use axum::{extract, extract::State, Json};
use crate::models::{ServerError,
                    AuthState,
                    LoginForm};

pub async fn login_user(
    State(state): State<AuthState>,
    payload: Json<LoginForm>,
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
        return Ok(());
    }
    Ok(())
}
