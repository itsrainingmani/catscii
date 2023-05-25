use serde::Deserialize;

#[tokio::main]
async fn main() {
    let art = get_cat_ascii_art().await.unwrap();
    println!("{art}");
}

async fn get_cat_ascii_art() -> color_eyre::Result<String> {
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

    let image_bytes = client
        .get(image.url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let image = image::load_from_memory(&image_bytes)?;
    let ascii_art = artem::convert(image, artem::options::OptionBuilder::new().build());

    Ok(ascii_art)
}
