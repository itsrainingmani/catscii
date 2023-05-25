use pretty_hex::PrettyHex;
use serde::Deserialize;

#[tokio::main]
async fn main() {
    let image_bytes = get_cat_image_bytes().await.unwrap();

    // Only dump the first 200 bytes
    println!("{:?}", &image_bytes[..200].hex_dump());
}

async fn get_cat_image_bytes() -> color_eyre::Result<Vec<u8>> {
    #[derive(Deserialize)]
    struct CatImage {
        url: String,
    }

    let api_url = "https://api.thecatapi.com/v1/images/search";
    let client = reqwest::Client::default();

    let image = client
        .get(api_url)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<CatImage>>()
        .await?
        .pop()
        .ok_or_else(|| color_eyre::eyre::eyre!("The Cat API returned no images"))?;

    Ok(client
        .get(image.url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec())
}
