use std::sync::LazyLock;

use spec_api::spec_provider_client::SpecProviderClient;
use tonic::transport::{Channel, Endpoint};

use crate::config::Config;

mod get_spec;
mod list_specs;

pub use get_spec::{GetSpecError, get_spec};
pub use list_specs::{ListSpecsError, list_specs};

/// Shared so every call reuses one connection to the spec-provider, which is
/// established on first use and again after it drops.
static CLIENT: LazyLock<SpecProviderClient<Channel>> =
    LazyLock::new(|| lazy_client(Config::spec_provider_url()));

/// A client of the spec-provider at `endpoint` that connects with its first call.
///
/// Clones of the client share the connection.
fn lazy_client(endpoint: &str) -> SpecProviderClient<Channel> {
    let channel = Endpoint::from_shared(endpoint.to_string())
        .expect("SPEC_PROVIDER_URL must be a valid uri")
        .connect_lazy();
    SpecProviderClient::new(channel)
}
