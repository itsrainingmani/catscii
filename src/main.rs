use axum::{
    body::BoxBody,
    extract::State,
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use reqwest::StatusCode;
use serde::Deserialize;
use std::{net::IpAddr, str::FromStr};

use tracing::{info, warn, Level};
use tracing_subscriber::{filter::Targets, layer::SubscriberExt, util::SubscriberInitExt};

use opentelemetry::{
    global,
    trace::{get_active_span, FutureExt, Span, Status, TraceContextExt, Tracer},
    Context, KeyValue,
};

use locat::Locat;
use std::sync::Arc;

#[derive(Clone)]
struct ServerState {
    client: reqwest::Client,
    locat: Arc<Locat>,
}

#[tokio::main]
async fn main() {
    // Sentry DSN is set in the .envrc which is encrypted using git-crypt
    let _guard = sentry::init((
        std::env::var("SENTRY_DSN").expect("$SENTRY_DSN must be set"),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    let (_honeyguard, _tracer) = opentelemetry_honeycomb::new_pipeline(
        std::env::var("HONEYCOMB_API_KEY").expect("$HONEYCOMB_API_KEY should be set"),
        "catscii".into(),
    )
    .install()
    .unwrap();

    // Install a tracing handler and log the address we are listening on
    let filter = Targets::from_str(std::env::var("RUST_LOG").as_deref().unwrap_or("info"))
        .expect("RUST_LOG should be a valid tracing filter");

    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .json()
        .finish()
        .with(filter)
        .init();

    let country_db_env_var = "GEOLITE2_COUNTRY_DB";
    let country_db_path = std::env::var(country_db_env_var)
        .unwrap_or_else(|_| panic!("${country_db_env_var} must be set"));

    let state = ServerState {
        client: Default::default(),
        locat: Arc::new(Locat::new(&country_db_path, "todo_analytics.db").unwrap()),
    };

    let app = Router::new()
        .route("/", get(root_get))
        .route("/panic", get(|| async { panic!("This is a test panic") }))
        .with_state(state);

    // Graceful shutdown
    let quit_sig = async {
        _ = tokio::signal::ctrl_c().await;
        warn!("Initiating graceful shutdown");
    };

    let addr = "0.0.0.0:8080".parse().unwrap();
    info!("Listening on {addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(quit_sig)
        .await
        .unwrap();
}

fn get_client_addr(headers: &HeaderMap) -> Option<IpAddr> {
    let header = headers.get("fly-client-ip")?;
    let header = header.to_str().ok()?;
    let addr = header.parse::<IpAddr>().ok()?;
    Some(addr)
}

async fn root_get(headers: HeaderMap, State(state): State<ServerState>) -> Response<BoxBody> {
    let tracer = global::tracer("");
    let mut span = tracer.start("root_get");
    span.set_attribute(KeyValue::new(
        "user_agent",
        headers
            .get(header::USER_AGENT)
            .map(|h| h.to_str().unwrap_or_default().to_owned())
            .unwrap_or_default(),
    ));

    if let Some(addr) = get_client_addr(&headers) {
        match state.locat.ip_to_iso_code(addr) {
            Some(country) => {
                info!("Got request from {country}");
                span.set_attribute(KeyValue::new("country", country.to_string()));
            }
            None => warn!("Could not determine country for IP Address"),
        }
    }

    root_get_inner(state)
        .with_context(Context::current_with_span(span))
        .await
}

async fn root_get_inner(state: ServerState) -> Response<BoxBody> {
    let tracer = global::tracer("");

    match get_cat_ascii_art(&state.client)
        .with_context(Context::current_with_span(
            tracer.start("get_cat_ascii_art"),
        ))
        .await
    {
        Ok(art) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            art,
        )
            .into_response(),
        Err(e) => {
            get_active_span(|span| {
                span.set_status(Status::Error {
                    description: format!("{e}").into(),
                })
            });
            (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong").into_response()
        }
    }
}

async fn get_cat_ascii_art(client: &reqwest::Client) -> color_eyre::Result<String> {
    let tracer = global::tracer("");

    #[derive(Deserialize)]
    struct CatImage {
        url: String,
    }

    let random_cat = client
        .get("https://api.thecatapi.com/v1/images/search")
        .send()
        .with_context(Context::current_with_span(tracer.start("api_headers")))
        .await?
        .error_for_status()?
        .json::<Vec<CatImage>>()
        .with_context(Context::current_with_span(tracer.start("api_body")))
        .await?
        .pop()
        .ok_or_else(|| color_eyre::eyre::eyre!("The Cat API returned no images"))?;

    let image_bytes = download_file(client, &random_cat.url)
        .with_context(Context::current_with_span(tracer.start("download_file")))
        .await?;

    let image = tracer.in_span("image::load_from_memory", |cx| {
        let img = image::load_from_memory(&image_bytes)?;

        cx.span()
            .set_attribute(KeyValue::new("width", img.width() as i64));
        cx.span()
            .set_attribute(KeyValue::new("height", img.height() as i64));

        Ok::<_, color_eyre::eyre::Report>(img)
    })?;

    let ascii_art = tracer.in_span("artem::convert", |_cx| {
        artem::convert(
            image,
            artem::options::OptionBuilder::new()
                .target(artem::options::TargetType::HtmlFile(true, true))
                .build(),
        )
    });

    Ok(ascii_art)
}

async fn download_file(client: &reqwest::Client, url: &str) -> color_eyre::Result<Vec<u8>> {
    let bytes = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    Ok(bytes.to_vec())
}
