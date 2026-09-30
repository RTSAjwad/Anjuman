//! Performs `crux_kv` operations for the Leptos (WASM) shell via `localStorage`.
//!
//! The core hands us a `crux_kv::protocol::KeyValueOperation`; we run it against
//! `gloo_storage::LocalStorage` and hand back a `crux_kv::KeyValueResult`. Token
//! bytes are stored as UTF-8 strings (the JWT is ASCII), so `String` round-trips
//! the `Vec<u8>` the core uses.

use shared::crux_kv::{
    KeyValueError, KeyValueOperation, KeyValueResponse, KeyValueResult,
    protocol::value::Value,
};
use gloo_storage::{LocalStorage, Storage};

/// Whether a key is present must be answered for `Get`/`Set`/`Delete` (their
/// responses carry the *previous* value, which is `Value::None` when absent).
fn read_string(key: &str) -> Option<Vec<u8>> {
    LocalStorage::get::<String>(key)
        .ok()
        .map(|s| s.into_bytes())
}

pub async fn execute(op: &KeyValueOperation) -> KeyValueResult {
    match op {
        KeyValueOperation::Get { key } => {
            let value = match read_string(key) {
                Some(bytes) => Value::Bytes(bytes),
                None => Value::None,
            };
            KeyValueResult::Ok {
                response: KeyValueResponse::Get { value },
            }
        }

        KeyValueOperation::Set { key, value } => {
            let previous = match read_string(key) {
                Some(bytes) => Value::Bytes(bytes),
                None => Value::None,
            };
            let as_string = String::from_utf8_lossy(value).into_owned();
            match LocalStorage::set(key, &as_string) {
                Ok(()) => KeyValueResult::Ok {
                    response: KeyValueResponse::Set { previous },
                },
                Err(e) => KeyValueResult::Err {
                    error: KeyValueError::Other {
                        message: e.to_string(),
                    },
                },
            }
        }

        KeyValueOperation::Delete { key } => {
            let previous = match read_string(key) {
                Some(bytes) => Value::Bytes(bytes),
                None => Value::None,
            };
            LocalStorage::delete(key);
            KeyValueResult::Ok {
                response: KeyValueResponse::Delete { previous },
            }
        }

        // Not used by the auth flow; treat as a no-op error rather than panic.
        other => KeyValueResult::Err {
            error: KeyValueError::Other {
                message: format!("unsupported kv operation: {other:?}"),
            },
        },
    }
}
