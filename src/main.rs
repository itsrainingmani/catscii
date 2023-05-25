use serde::Deserialize;

#[tokio::main]
async fn main() {
    let res = reqwest::get("https://api.thecatapi.com/v1/images/search")
        .await
        .unwrap();

    if !res.status().is_success() {
        panic!("Request failed with HTTP {}", res.status());
    }

    #[derive(Deserialize)]
    struct CatImage {
        url: String,
    }

    let images: Vec<CatImage> = res.json().await.unwrap();
    let image = images
        .first()
        .expect("the cat API should return atleast one image");

    println!("The image is at {}", image.url);
}
