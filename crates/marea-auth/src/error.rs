//! Structured PocketBase errors.
//!
//! PocketBase puts the useful detail in per-field `data` entries (e.g.
//! `data.email = { code: "validation_not_unique", message: "Value must be
//! unique." }`). We keep the **field name + code** so the UI can say exactly
//! which field failed and localize precisely by `(field, code)`, and also
//! build a human fallback string.

use serde::Deserialize;

/// A single field validation error, with the field name lifted out of the
/// `data` map key. Lets callers localize precisely by `(field, code)` instead
/// of guessing from the message text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    pub field: String,
    pub code: String,
    pub message: String,
}

#[derive(thiserror::Error, Debug)]
pub enum PocketBaseError {
    #[error("network: {0}")]
    Network(#[from] reqwest::Error),
    /// An auth/validation failure. `message` is a human fallback ("Email:
    /// Value must be unique."); `fields` carries the structured per-field
    /// errors (empty when PocketBase sent only a top-level message).
    #[error("auth: {message}")]
    Auth {
        message: String,
        fields: Vec<FieldError>,
    },
    #[error("decode response: {message} (body: {body})")]
    Decode { body: String, message: String },
    #[error("crypto: {0}")]
    Crypto(String),
}

impl PocketBaseError {
    /// A clean, user-facing message — no dev-oriented `auth:`/`network:`
    /// prefix and no raw decode bodies. The UI shows this (or a localized
    /// override keyed on the structured fields) instead of `to_string()`.
    pub fn user_message(&self) -> String {
        match self {
            // `reqwest::Error::is_connect/is_timeout` aren't available on
            // wasm, and the reachability-vs-other distinction adds little, so
            // one clear message covers any transport failure.
            PocketBaseError::Network(_) => {
                "Couldn't reach the server. Check your connection and try again.".to_string()
            }
            PocketBaseError::Auth { message, .. } => message.clone(),
            PocketBaseError::Decode { .. } => {
                "The server returned an unexpected response. Please try again.".to_string()
            }
            PocketBaseError::Crypto(_) => {
                "Couldn't set up your secure session. Please try again.".to_string()
            }
        }
    }

    /// True for transport-level failures (connection refused, DNS, timeout)
    /// that indicate the device is offline — as opposed to the server actually
    /// responding with an error.
    pub fn is_network_error(&self) -> bool {
        matches!(self, PocketBaseError::Network(_))
    }
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    #[serde(default)]
    message: Option<String>,
    /// Per-field validation errors keyed by field name. Where PocketBase puts
    /// the actual reason (the top-level `message` is generic).
    #[serde(default)]
    data: std::collections::HashMap<String, FieldErrorRaw>,
}

/// The `{ code, message }` value PocketBase nests under each `data` field.
#[derive(Deserialize)]
struct FieldErrorRaw {
    #[serde(default)]
    code: String,
    #[serde(default)]
    message: String,
}

/// Parse a PocketBase error body into a structured [`PocketBaseError::Auth`].
pub(crate) fn auth_error_from_body(body: &str) -> PocketBaseError {
    let env = serde_json::from_str::<ErrorEnvelope>(body).ok();

    let mut fields: Vec<FieldError> = env
        .as_ref()
        .map(|e| {
            e.data
                .iter()
                .map(|(field, raw)| FieldError {
                    field: field.clone(),
                    code: raw.code.clone(),
                    message: raw.message.trim().to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    fields.sort_by(|a, b| a.field.cmp(&b.field));

    let message = if !fields.is_empty() {
        // "Email: Value must be unique." — names the field so the message
        // stands on its own even before the UI localizes it.
        fields
            .iter()
            .map(|f| {
                let detail = if f.message.is_empty() {
                    f.code.as_str()
                } else {
                    f.message.as_str()
                };
                format!("{}: {}", humanize_field(&f.field), detail)
            })
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        env.and_then(|e| e.message)
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| body.to_string())
    };

    PocketBaseError::Auth { message, fields }
}

/// Title-case a PocketBase field name for the fallback message.
fn humanize_field(field: &str) -> String {
    match field {
        "email" => "Email".to_string(),
        "password" => "Password".to_string(),
        "passwordConfirm" => "Password confirmation".to_string(),
        other => {
            let mut c = other.chars();
            match c.next() {
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_error_names_the_field_and_keeps_code() {
        // "Value must be unique." alone doesn't say which field — the
        // structured form must carry field=email + the code, and the fallback
        // names it.
        let body = r#"{"code":400,"message":"Failed to create record.","data":{"email":{"code":"validation_not_unique","message":"Value must be unique."}}}"#;
        let PocketBaseError::Auth { message, fields } = auth_error_from_body(body) else {
            panic!("expected Auth");
        };
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field, "email");
        assert_eq!(fields[0].code, "validation_not_unique");
        assert!(
            message.contains("Email"),
            "fallback names the field: {message}"
        );
        assert!(message.contains("Value must be unique"), "got: {message}");
        assert!(
            !message.contains("Failed to create record"),
            "got: {message}"
        );
    }

    #[test]
    fn auth_error_collects_multiple_fields_sorted() {
        let body = r#"{"message":"x","data":{"password":{"code":"validation_length_out_of_range","message":"Too short."},"email":{"code":"validation_not_unique","message":"Value must be unique."}}}"#;
        let PocketBaseError::Auth { fields, .. } = auth_error_from_body(body) else {
            panic!("expected Auth");
        };
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].field, "email", "fields sorted for determinism");
        assert_eq!(fields[1].field, "password");
    }

    #[test]
    fn auth_error_falls_back_to_top_level_when_no_data() {
        let PocketBaseError::Auth { message, fields } =
            auth_error_from_body(r#"{"message":"Failed to authenticate."}"#)
        else {
            panic!("expected Auth");
        };
        assert!(fields.is_empty());
        assert_eq!(message, "Failed to authenticate.");
    }

    #[test]
    fn auth_error_falls_back_to_raw_body_on_garbage() {
        let PocketBaseError::Auth { message, fields } = auth_error_from_body("<html>502</html>")
        else {
            panic!("expected Auth");
        };
        assert!(fields.is_empty());
        assert_eq!(message, "<html>502</html>");
    }

    #[test]
    fn user_message_has_no_dev_prefix() {
        let m = PocketBaseError::Auth {
            message: "Email: Value must be unique.".into(),
            fields: vec![],
        }
        .user_message();
        assert_eq!(m, "Email: Value must be unique.");
        assert!(
            !PocketBaseError::Decode {
                body: "garbage".into(),
                message: "x".into()
            }
            .user_message()
            .contains("garbage")
        );
    }
}
