pub mod repositories;
pub mod sessions;

use axum::{Json, Router};
use sec_store::repository::remote::ErrorResponse;
use serde::Serialize;

use crate::{ApiError, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(repositories::router())
        .merge(sessions::router())
}

#[derive(Debug, Serialize)]
pub(crate) struct SimpleStatus {
    status: &'static str,
}

impl SimpleStatus {
    pub(crate) fn new(status: &'static str) -> Self {
        Self { status }
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_errors_are_sanitized_in_responses() {
        let error = ApiError::internal("leaked filesystem path");
        assert_eq!(error.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.message, "Internal server error");
    }
}
