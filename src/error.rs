use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("rpc error: {0}")]
    Rpc(#[from] anyhow::Error),

    #[error("grpc error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("other error: {0}")]
    Other(String),
}
