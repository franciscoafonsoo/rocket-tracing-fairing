#![deny(unsafe_code, clippy::all)]

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate tracing;

use rocket::http::Status;
use rocket::serde::{json::Json, Serialize};

use tracing_log::LogTracer;

// Spans
use rocket_tracing_fairing::logging::{
    default_logging_layer, filter_layer, json_logging_layer, LogLevel, LogType,
};
use rocket_tracing_fairing::spans::{RequestId, TracingFairing, TracingSpan};

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OutputData<'a> {
    pub message: &'a str,
    pub request_id: String,
}

#[get("/abc")]
pub async fn abc<'a>(
    span: TracingSpan,
    request_id: RequestId,
) -> Result<Json<OutputData<'a>>, (Status, Json<OutputData<'a>>)> {
    let entered = span.0.enter();
    info!("Hello World");

    let mock_data = OutputData {
        message: "Hello World",
        request_id: request_id.0,
    };
    span.0.record(
        "output",
        serde_json::to_string(&mock_data).unwrap().as_str(),
    );
    drop(entered);
    Err((Status::NotFound, Json(mock_data)))
}

// Rocket setup

#[launch]
fn rocket() -> _ {
    use tracing_subscriber::prelude::*;

    LogTracer::init().expect("Unable to setup log tracer!");

    let log_type =
        LogType::from(std::env::var("LOG_TYPE").unwrap_or_else(|_| "formatted".to_string()));
    let log_level = LogLevel::from(
        std::env::var("LOG_LEVEL")
            .unwrap_or_else(|_| "normal".to_string())
            .as_str(),
    );

    match log_type {
        LogType::Formatted => {
            tracing::subscriber::set_global_default(
                tracing_subscriber::registry()
                    .with(default_logging_layer())
                    .with(filter_layer(log_level)),
            )
            .unwrap();
        }
        LogType::Json => {
            tracing::subscriber::set_global_default(
                tracing_subscriber::registry()
                    .with(json_logging_layer())
                    .with(filter_layer(log_level)),
            )
            .unwrap();
        }
    };

    rocket::build()
        .mount("/", routes![abc])
        .attach(TracingFairing)
}
