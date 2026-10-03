use uuid::Uuid;

use crate::{
    application::password::{
        hash::{HashPasswordError, hash_password},
        verify::{VerifyPasswordError, verify_password},
    },
    persistence::users::{
        find_by_id::{FindByIdUserError, find_user_by_id},
        save::{SaveUserError, save_user},
    },
};

pub enum SetUserPasswordError {
    UserNotFound,
    HashingError,
    InvalidCredentials,
    DatabaseError,
}

pub async fn set_user_password(
    user_id: Uuid,
    current_password: &str,
    new_password: &str,
) -> Result<(), SetUserPasswordError> {
    let user = find_user_by_id(user_id).await.map_err(|e| match e {
        FindByIdUserError::DatabaseError => SetUserPasswordError::DatabaseError,
        FindByIdUserError::InvalidData => SetUserPasswordError::DatabaseError,
    })?;

    let Some(mut user) = user else {
        return Err(SetUserPasswordError::UserNotFound);
    };

    // ==== Verify the old password ====
    verify_password(current_password, user.password_hash()).map_err(|e| match e {
        VerifyPasswordError::HashingError => SetUserPasswordError::HashingError,
        VerifyPasswordError::InvalidPassword => SetUserPasswordError::InvalidCredentials,
    })?;

    // ==== Set the new password ====
    let hashed_password = hash_password(new_password).map_err(|e| match e {
        HashPasswordError::HashingError => SetUserPasswordError::HashingError,
    })?;
    user.set_password_hash(hashed_password);

    save_user(&user).await.map_err(|e| match e {
        SaveUserError::DatabaseError => SetUserPasswordError::DatabaseError,
        SaveUserError::UserNotFound => SetUserPasswordError::DatabaseError,
        // This case should not happen since we just fetched the user, and don't change the user name.
        SaveUserError::UserNameAlreadyExists => SetUserPasswordError::DatabaseError,
    })?;

    Ok(())
}
