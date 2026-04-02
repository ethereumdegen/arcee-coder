use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Rate limited: retry after {retry_after_secs:?}s")]
    RateLimit { retry_after_secs: Option<u64> },

    #[error("Server error: {0}")]
    Server(String),

    #[error("Context too long: {0}")]
    ContextTooLong(String),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Stream parse error: {0}")]
    StreamParse(String),

    #[error("API error: {error_type}: {message}")]
    ApiResponse {
        error_type: String,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    RateLimit,
    ServerError,
    AuthError,
    ContextError,
    Other,
}

impl ApiError {
    pub fn category(&self) -> ErrorCategory {
        match self {
            ApiError::Auth(_) => ErrorCategory::AuthError,
            ApiError::RateLimit { .. } => ErrorCategory::RateLimit,
            ApiError::Server(_) => ErrorCategory::ServerError,
            ApiError::ContextTooLong(_) => ErrorCategory::ContextError,
            ApiError::Request(e) => {
                if e.is_timeout() || e.is_connect() {
                    ErrorCategory::ServerError
                } else if e.status().is_some_and(|s| s == 429) {
                    ErrorCategory::RateLimit
                } else if e.status().is_some_and(|s| s == 401 || s == 403) {
                    ErrorCategory::AuthError
                } else if e.status().is_some_and(|s| s.is_server_error()) {
                    ErrorCategory::ServerError
                } else {
                    ErrorCategory::Other
                }
            }
            ApiError::StreamParse(_) => ErrorCategory::Other,
            ApiError::ApiResponse { error_type, .. } => match error_type.as_str() {
                "authentication_error" | "permission_error" => ErrorCategory::AuthError,
                "rate_limit_error" => ErrorCategory::RateLimit,
                "overloaded_error" | "api_error" => ErrorCategory::ServerError,
                "invalid_request_error" => ErrorCategory::Other,
                _ => ErrorCategory::Other,
            },
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self.category(),
            ErrorCategory::RateLimit | ErrorCategory::ServerError
        )
    }
}
