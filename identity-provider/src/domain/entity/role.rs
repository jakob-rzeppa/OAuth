use uuid::Uuid;

#[derive(Debug)]
pub enum RoleError {
    EmptyId,
    InvalidName,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Role {
    id: Uuid,
    name: String,
    description: String,
}

impl Role {
    pub fn new(id: Uuid, name: String, description: String) -> Result<Self, RoleError> {
        if id.is_nil() {
            return Err(RoleError::EmptyId);
        }

        if name.is_empty() {
            return Err(RoleError::InvalidName);
        }

        Ok(Role {
            id,
            name,
            description,
        })
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_nil_id() {
        let id = Uuid::nil();
        let name = "Admin".to_string();
        let description = "Administrator role".to_string();

        let result = Role::new(id, name, description);
        assert!(matches!(result, Err(RoleError::EmptyId)));
    }

    #[test]
    fn rejects_empty_name() {
        let id = Uuid::new_v4();
        let name = "".to_string();
        let description = "Administrator role".to_string();

        let result = Role::new(id, name, description);
        assert!(matches!(result, Err(RoleError::InvalidName)));
    }
}
