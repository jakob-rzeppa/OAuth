use std::sync::LazyLock;

use uuid::Uuid;

use crate::domain::client::Client;

#[derive(serde::Deserialize)]
struct SelfClientRow {
    id: Uuid,
    client_name: String,
    redirect_uris: Option<Vec<String>>,
    scopes: Vec<String>,
}

#[derive(serde::Deserialize)]
struct SelfStore {
    client: SelfClientRow,
}

static SELF_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let self_yaml_data =
        std::fs::read_to_string("seed/self.yaml").expect("Failed to read self.yaml");
    let self_data: SelfStore =
        serde_yaml::from_str(&self_yaml_data).expect("Failed to parse self.yaml");
    let row = self_data.client;

    Client::new(
        row.id,
        row.client_name,
        row.redirect_uris.unwrap_or_default(),
        row.scopes,
    )
});

#[fnmock::fakeable]
pub fn get_auth_client() -> Client {
    SELF_CLIENT.clone()
}
