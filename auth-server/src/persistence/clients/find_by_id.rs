use uuid::Uuid;

use crate::{domain::client::Client, persistence::clients::store::clients};

#[fnmock::fakeable]
pub fn find_client_by_id(id: &Uuid) -> Option<Client> {
    let clients = clients();
    clients.iter().find(|client| client.id() == id).cloned()
}
