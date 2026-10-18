use crate::{domain::entity::role::Role, persistence::roles::store::roles};

pub fn find_all_roles() -> Vec<Role> {
    roles().to_vec()
}
