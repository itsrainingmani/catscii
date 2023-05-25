#[tokio::main]
async fn main() {
    let res = reqwest::get("https://api.thecatapi.com/v1/images/search")
        .await
        .unwrap();

    println!("Status: {}", res.status());
    let body = res.text().await.unwrap();
    println!("Body: {}", body);
}
