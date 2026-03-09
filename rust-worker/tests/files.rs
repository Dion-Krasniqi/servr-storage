#[path = "common/mod.rs"]
mod common;
use common::spawn_app;

const USER_UUID: &str = "7c590022-c579-4e69-8eb4-92e67440f93f";

#[tokio::test]
async fn test_get_files() {
    let app = spawn_app().await;

    let res = app.client
        .post(format!("{}/get-files", app.base_url))
        .json(&serde_json::json!({"owner_id":USER_UUID}))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body.is_array());
}

// root folder
#[tokio::test]
async fn test_create_folder() {
    let app = spawn_app().await;

    let res = app.client
        .post(format!("{}/create-folder", app.base_url))
        .json(&serde_json::json!({"owner_id":USER_UUID,
                                 "folder_name":"Folder1",
                                 "parent_id":"",}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}

async fn test_upload_file(){}

async fn test_delete_file(){}

async fn test_rename_file(){}

