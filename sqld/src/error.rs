use axum::response::IntoResponse;
use hyper::StatusCode;

use crate::{
    auth::AuthError, query_result_builder::QueryResultBuilderError,
    replication::replica::error::ReplicationError,
};

#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("LibSQL failed to bind provided query parameters: `{0}`")]
    LibSqlInvalidQueryParams(anyhow::Error),
    #[error("Transaction timed-out")]
    LibSqlTxTimeout,
    #[error("Server can't handle additional transactions")]
    LibSqlTxBusy,
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    RusqliteError(#[from] rusqlite::Error),
    #[error("Failed to execute query via RPC. Error code: {}, message: {}", .0.code, .0.message)]
    RpcQueryError(crate::rpc::proxy::rpc::Error),
    #[error("Failed to execute queries via RPC protocol: `{0}`")]
    RpcQueryExecutionError(tonic::Status),
    #[error("Database value error: `{0}`")]
    DbValueError(String),
    // Dedicated for most generic internal errors. Please use it sparingly.
    // Consider creating a dedicate enum value for your error.
    #[error("Internal Error: `{0}`")]
    Internal(String),
    #[error("Invalid batch step: {0}")]
    InvalidBatchStep(usize),
    #[error("Not authorized to execute query: {0}")]
    NotAuthorized(String),
    #[error("The replicator exited, instance cannot make any progress.")]
    ReplicatorExited,
    #[error("Timed out while openning database connection")]
    DbCreateTimeout,
    #[error(transparent)]
    BuilderError(#[from] QueryResultBuilderError),
    #[error("Operation was blocked{}", .0.as_ref().map(|msg| format!(": {}", msg)).unwrap_or_default())]
    Blocked(Option<String>),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("Too many concurrent requests")]
    TooManyRequests,
    #[error("Failed to parse query: `{0}`")]
    FailedToParse(String),
    #[error("Query error: `{0}`")]
    QueryError(String),
    #[error("Unauthorized: `{0}`")]
    AuthError(#[from] AuthError),
    // Catch-all error since we use anyhow in certain places
    #[error("Internal Error: `{0}`")]
    Anyhow(#[from] anyhow::Error),
    #[error("Invalid host header: `{0}`")]
    InvalidHost(String),
    #[error("Namespace `{0}` doesn't exist")]
    NamespaceDoesntExist(String),
    #[error("Namespace `{0}` already exists")]
    NamespaceAlreadyExist(String),
    #[error("Invalid namespace")]
    InvalidNamespace,
    #[error("replication error: {0}")]
    ReplicationError(#[from] ReplicationError),
    #[error("Failed to connect to primary")]
    PrimaryConnectionTimeout,
    #[error("Cannot load a dump on a replica")]
    ReplicaLoadDump,
    #[error("cannot load from a dump if a database already exists.")]
    LoadDumpExistingDb,
}

impl Error {
    fn format_err(&self, status: StatusCode) -> axum::response::Response {
        let json = serde_json::json!({ "error": self.to_string() });
        tracing::error!("HTTP API: {}, {}", status, json);
        (status, axum::Json(json)).into_response()
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        use Error::*;

        match &self {
            FailedToParse(_) => self.format_err(StatusCode::BAD_REQUEST),
            AuthError(_) => self.format_err(StatusCode::UNAUTHORIZED),
            Anyhow(_) => self.format_err(StatusCode::INTERNAL_SERVER_ERROR),
            LibSqlInvalidQueryParams(_) => self.format_err(StatusCode::BAD_REQUEST),
            LibSqlTxTimeout => self.format_err(StatusCode::BAD_REQUEST),
            LibSqlTxBusy => self.format_err(StatusCode::TOO_MANY_REQUESTS),
            IOError(_) => self.format_err(StatusCode::INTERNAL_SERVER_ERROR),
            RusqliteError(_) => self.format_err(StatusCode::INTERNAL_SERVER_ERROR),
            RpcQueryError(_) => self.format_err(StatusCode::BAD_REQUEST),
            RpcQueryExecutionError(_) => self.format_err(StatusCode::INTERNAL_SERVER_ERROR),
            DbValueError(_) => self.format_err(StatusCode::BAD_REQUEST),
            Internal(_) => self.format_err(StatusCode::INTERNAL_SERVER_ERROR),
            InvalidBatchStep(_) => self.format_err(StatusCode::INTERNAL_SERVER_ERROR),
            NotAuthorized(_) => self.format_err(StatusCode::UNAUTHORIZED),
            ReplicatorExited => self.format_err(StatusCode::SERVICE_UNAVAILABLE),
            DbCreateTimeout => self.format_err(StatusCode::SERVICE_UNAVAILABLE),
            BuilderError(_) => self.format_err(StatusCode::INTERNAL_SERVER_ERROR),
            Blocked(_) => self.format_err(StatusCode::INTERNAL_SERVER_ERROR),
            Json(_) => self.format_err(StatusCode::INTERNAL_SERVER_ERROR),
            TooManyRequests => self.format_err(StatusCode::TOO_MANY_REQUESTS),
            QueryError(_) => self.format_err(StatusCode::BAD_REQUEST),
            InvalidHost(_) => self.format_err(StatusCode::BAD_REQUEST),
            NamespaceDoesntExist(_) => self.format_err(StatusCode::BAD_REQUEST),
            ReplicationError(_) => self.format_err(StatusCode::INTERNAL_SERVER_ERROR),
            PrimaryConnectionTimeout => self.format_err(StatusCode::INTERNAL_SERVER_ERROR),
            NamespaceAlreadyExist(_) => self.format_err(StatusCode::BAD_REQUEST),
            InvalidNamespace => self.format_err(StatusCode::BAD_REQUEST),
            ReplicaLoadDump => self.format_err(StatusCode::BAD_REQUEST),
            LoadDumpExistingDb => self.format_err(StatusCode::BAD_REQUEST),
        }
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for Error {
    fn from(inner: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::Internal(format!(
            "Failed to receive response via oneshot channel: {inner}"
        ))
    }
}

impl From<bincode::Error> for Error {
    fn from(other: bincode::Error) -> Self {
        Self::Internal(other.to_string())
    }
}
