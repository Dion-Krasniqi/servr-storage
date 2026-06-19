use axum::{extract, extract::State, Json, http::StatusCode};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use crate::models::{ServerError,
                    AppState,
                    SignInForm,
                    SignUpForm,
                    TestToken,
                    Claims};
use jsonwebtoken::{encode, decode, Header, Algorithm, EncodingKey,
                   DecodingKey, Validation};
use sha2::{Sha256, Digest};
use uuid::Uuid;
use crate::methods::create_bucket_func;
use sqlx::Acquire;

fn hash_algorithm(
    password: &str, 
) -> String {
    let hash = Sha256::digest(password);
    //format!("{:x}", hash)
    hash.iter().map(|a| format!("{:02x}", a)).collect()
}
// acts more like a session token for now
fn create_token(
    data: String,
    expires: u64,
    key: &str,
) -> String {
    let exp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap().as_secs() + expires;
    let claim = Claims { sub: data, exp };
    let token = encode(&Header::default(), 
        &claim, 
        &EncodingKey::from_secret(key.as_ref())
    ).unwrap();
    return token
}
pub async fn login_user(
    jar: CookieJar,
    State(state): State<AppState>,
    payload: Json<SignInForm>,
) -> Result<CookieJar, ServerError> {
    println!("{}", payload.email);
    let email = payload.email.clone();
    let user: Option<(Uuid, String, bool)> = 
        sqlx::query_as(r#"SELECT user_id,hashed_password,active from users
                          WHERE email = ($1);"#)
        .bind(&email)
        .fetch_optional(&state.pool)
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
    let user_password = hash_algorithm(&payload.password);
    if !(user_password == hashed_password) { 
        return Err(ServerError::InternalError("User password does not match".to_string()));
    }
    let token = create_token(user_id.to_string(), 300, &state.key);
    let cookie = Cookie::build(("session", token))
        .path("/")
        .http_only(true)
        .secure(false)
        .build();
    Ok(jar.add(cookie))
}
pub async fn read_me(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<String, ServerError> { 
    let encd_token: String = if let Some(session_id) = jar.get("session") {
        session_id.to_string()
    } else {
        return Err(
            ServerError::Unauthorized("No session token found".to_string()));
    };
    let user: Claims = decode(&encd_token, 
        &DecodingKey::from_secret(&state.key.as_ref()), 
        &Validation::default()).unwrap().claims;

    Ok(user.sub)
}
pub async fn get_current_user(
    jar: CookieJar,
    key: &str,
) -> Result<String, ServerError> { 
    /*let SECRET_KEY: String = match std::env::var("SECRET_KEY") {
        Ok(key) => key,
        Err(e) => {
            "".to_string()
        },
    };
    */
    let encd_token: String = if let Some(session_id) = jar.get("session") {
        session_id.value().to_string()
    } else {
        return Err(
            ServerError::Unauthorized("No session token found".to_string()));
    };
    let user: Claims = match decode(&encd_token, 
        &DecodingKey::from_secret(key.as_ref()), 
        &Validation::default()) {
        Ok(val) => val.claims,
        Err(e) => { 
            eprintln!("{}", e);
            return Err(
            ServerError::InternalError("User already exists".to_string()))
        },
    };                    
    
    Ok(user.sub)
}
pub async fn logout_user(
    jar: CookieJar,
) -> Result<CookieJar, ServerError> {
    Ok(jar.remove(Cookie::from("session")))
}
pub async fn create_user(
    State(state): State<AppState>,
    payload: Json<SignUpForm>,
) -> Result<StatusCode, ServerError> {
    println!("{}", payload.email);
    let email = payload.email.clone();
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
    let mut conn = state.pool.acquire().await.map_err(|e| ServerError::DatabaseError(e.to_string()))?;
    let mut tx = conn.begin().await.map_err(|e| ServerError::DatabaseError(e.to_string()))?;

    sqlx::query(r#"INSERT INTO users (user_id, 
    email, hashed_password, active, super_user, storage_used)
                   VALUES ($1, $2, $3, $4, $5, $6);"#)
        .bind(&user_id)
        .bind(&email)
        .bind(&hashed_password)
        .bind(true)
        .bind(false)
        .bind(0)
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerError::DatabaseError(format!("Failed to create user. Error: {}", e)))?;
   
    create_bucket_func(state.client, &user_id.to_string())
        .await.map_err(|e| ServerError::InternalError("Failed to create user bucket".to_string()))?;
    tx.commit()
        .await.map_err(|e| ServerError::DatabaseError(e.to_string()))?;
        /*Ok() => {},
        _ => return Err(
                ServerError::InternalError("Failed to create user bucket".to_string())),
        }*/

        
    Ok(StatusCode::CREATED)
}

pub async fn login_test(
    State(state): State<AppState>,
    payload: Json<SignInForm>,
) -> Result<Json<String>, ServerError> {
    let email = payload.email.trim();
    let user: Option<(String,String, bool)> = 
        sqlx::query_as(r#"SELECT user_id,hashed_password,active from users
                          WHERE email = ($1);"#)
        .bind(&email)
        .fetch_optional(&state.pool)
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
    let user_password = hash_algorithm(&payload.password);
    if !(user_password == hashed_password) { 
        return Err(ServerError::InternalError("User password does not match".to_string()));
    }
    let token = create_token(user_id, 300, &state.key);
    Ok(Json(token))
}
pub async fn get_current_test(
    State(state): State<AppState>,
    payload: Json<TestToken>,
) -> Result<String, ServerError> { 
    let user: Claims = decode(&payload.token, 
        &DecodingKey::from_secret(&state.key.as_ref()), 
        &Validation::default()).unwrap().claims;

    Ok(user.sub)
}

