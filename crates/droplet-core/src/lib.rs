pub mod diagnostics;
pub mod error;
pub mod model;
pub mod protocol;
pub mod security;
pub mod service;
pub mod storage;

pub use error::{AppError, ErrorCode, Result};
pub use service::Core;

pub fn export_types(directory: &std::path::Path) -> std::result::Result<(), ts_rs::ExportError> {
    use ts_rs::TS;
    let config = ts_rs::Config::default().with_out_dir(directory);
    protocol::MainRequest::export_all(&config)?;
    protocol::PetRequest::export_all(&config)?;
    protocol::Reply::export_all(&config)?;
    protocol::PetReply::export_all(&config)?;
    AppError::export_all(&config)
}
