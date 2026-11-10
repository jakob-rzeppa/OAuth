use std::sync::LazyLock;

use uuid::Uuid;

use crate::domain::client::Client;

#[derive(serde::Deserialize)]
struct ClientRow {
    id: Uuid,
    client_name: String,
    redirect_uris: Vec<String>,
    scopes: Vec<String>,
}

#[derive(serde::Deserialize)]
struct ClientStore {
    clients: Vec<ClientRow>,
}

static ROLES: LazyLock<Vec<Client>> = LazyLock::new(|| {
    let clients_yaml_data =
        std::fs::read_to_string("seed/clients.yaml").expect("Failed to read clients.yaml");
    let clients_data: ClientStore =
        serde_yaml::from_str(&clients_yaml_data).expect("Failed to parse clients.yaml");

    clients_data
        .clients
        .into_iter()
        .map(|row| Client::new(row.id, row.client_name, row.redirect_uris, row.scopes))
        .collect()
});

#[fnmock::fakeable]
fn clients() -> &'static [Client] {
    &ROLES
}

#[fnmock::fakeable]
pub fn find_client(id: &Uuid) -> Option<Client> {
    let clients = clients();
    clients.iter().find(|client| client.id() == id).cloned()
}
