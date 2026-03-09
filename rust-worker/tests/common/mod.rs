use tokio::net::TcpListener;
use rust_worker::setup::setup;

pub struct TestApp {
    pub base_url: String,
    pub client: reqwest::Client,
}

pub async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let app = setup().await.unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    TestApp {
        base_url: format!("http://127.0.0.1:{}", port),
        client: reqwest::Client::new(),
    }
}


