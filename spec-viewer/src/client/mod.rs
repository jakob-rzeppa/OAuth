//! The adapters that talk to the services behind the contracts of the application.

mod content_storage;
mod spec_provider;

pub use content_storage::HttpContentGateway;
pub use spec_provider::GrpcSpecGateway;
