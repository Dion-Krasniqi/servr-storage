
#[path = "common/mod.rs"]
mod common;
use common::spawn_app;

#[tokio::test]
async fn test_get_files(){
    let app = spawn_app().await;

    let res = app.client
        .post(format!("{}/get-files", app.base_url))
        .json(&serde_json::json!({"owner_id":"123"}))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body.is_array());
}

async fn test_create_folder(){}

async fn test_upload_file(){}

async fn test_delete_file(){}

async fn test_rename_file(){}

