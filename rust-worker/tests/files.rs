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

#[tokio::test]
async fn test_create_folder_wo_parent() {
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


#[tokio::test]
async fn test_delete_file(){
    //get this from another func for more control or get files and then rand
    let file_id = "123";
    
    let app = spawn_app().await;
    let res = app.client
        .post(format!("{}/delete-file", app.base_url))
        .json(&serde_json::json!({"owner_id":USER_UUID,
                                  "file_id":file_id}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn test_upload_file_wo_parent(){

    let app = spawn_app().await;
    
    let file = b"Hello World".to_vec();
    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(file)
            .file_name("test.txt")
            .mime_str("text/plain").unwrap())
        .text("user_id", USER_UUID)
        .text("parent_id", "");
    
    let res = app.client
        .post(format!("{}/upload-file", app.base_url))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
}


#[tokio::test]
async fn test_rename_file(){
    
    //get this from another func for more control or get files and then rand
    let file_id = "123";
    
    let app = spawn_app().await;

    let res = app.client
        .post(format!("{}/rename-file", app.base_url))
        .json(&serde_json::json!({"owner_id":USER_UUID,
                                  "file_id":file_id,
                                  "file_name": "NewName"}))
        .send()
        .await
        .unwrap();
    
    assert_eq!(res.status(), 200);
}



