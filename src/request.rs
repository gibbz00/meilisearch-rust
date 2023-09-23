use crate::{Error, MeilisearchCommunicationError, MeilisearchError};
use log::{error, trace, warn};
use serde::de::DeserializeOwned;
use serde_json::from_str;

#[derive(Debug)]
pub(crate) enum Method<Q, B> {
    Get { query: Q },
    Post { query: Q, body: B },
    Patch { query: Q, body: B },
    Put { query: Q, body: B },
    Delete { query: Q },
}

fn parse_response<Output: DeserializeOwned>(
    status_code: u16,
    expected_status_code: u16,
    body: &str,
    url: String,
) -> Result<Output, Error> {
    if status_code == expected_status_code {
        match from_str::<Output>(body) {
            Ok(output) => {
                trace!("Request succeed");
                return Ok(output);
            }
            Err(e) => {
                error!("Request succeeded but failed to parse response");
                return Err(Error::ParseError(e));
            }
        };
    }

    warn!(
        "Expected response code {}, got {}",
        expected_status_code, status_code
    );

    match from_str::<MeilisearchError>(body) {
        Ok(e) => Err(Error::from(e)),
        Err(e) => {
            if status_code >= 400 {
                return Err(Error::MeilisearchCommunication(
                    MeilisearchCommunicationError {
                        status_code,
                        message: None,
                        url,
                    },
                ));
            }
            Err(Error::ParseError(e))
        }
    }
}

pub fn qualified_version() -> String {
    const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

    format!("Meilisearch Rust (v{})", VERSION.unwrap_or("unknown"))
}

#[cfg(not(target_arch = "wasm32"))]
pub use native_client::add_query_parameters;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native_client::{request, stream_request};
#[cfg(not(target_arch = "wasm32"))]
mod native_client;

#[cfg(target_arch = "wasm32")]
pub use wasm_client::add_query_parameters;
#[cfg(target_arch = "wasm32")]
pub(crate) use wasm_client::request;
#[cfg(target_arch = "wasm32")]
mod wasm_client;
