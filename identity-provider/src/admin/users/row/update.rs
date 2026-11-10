use axum::extract::Path;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    admin::{
        layout::{
            form::HtmlForm,
            notice::{ErrorNotice, Notice},
        },
        users::{get_user, parse_user_id, row::UserRowWithNotice},
    },
    application::users::update::{self, UpdateUserError},
};

#[derive(Deserialize)]
pub struct UpdateUserForm {
    user_name: String,
    display_name: String,
    #[serde(default)]
    role_ids: Vec<Uuid>,
}

/// PUT /admin/users/{user_id}
pub async fn update_user(
    Path(user_id): Path<String>,
    HtmlForm(UpdateUserForm {
        user_name,
        display_name,
        role_ids,
    }): HtmlForm<UpdateUserForm>,
) -> Result<UserRowWithNotice, ErrorNotice> {
    let user_id = parse_user_id(&user_id)?;

    update::update_user(user_id, Some(user_name), Some(display_name), Some(role_ids))
        .await
        .map_err(|e| match e {
            UpdateUserError::UserNotFound => ErrorNotice::user_not_found(),
            UpdateUserError::InvalidUserName => ErrorNotice::invalid_user_name(),
            UpdateUserError::UserNameAlreadyExists => ErrorNotice::user_name_already_exists(),
            UpdateUserError::DatabaseError => ErrorNotice::internal(),
        })?;

    let user = get_user(user_id).await?;
    let notice = Notice::success(format!("Updated {}.", user.user_name));

    Ok(UserRowWithNotice::new(user, notice))
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use uuid::Uuid;

    use crate::{
        admin::users::testing::{form, send, user},
        persistence::{
            roles::find_by_ids::find_roles_by_ids_fake,
            users::{
                find_by_id::find_user_by_id_fake,
                save::{SaveUserError, save_user_fake},
            },
        },
    };

    #[tokio::test]
    async fn updates_a_user_and_returns_the_row() {
        let id = Uuid::new_v4();
        find_user_by_id_fake().setup(move |_| Ok(Some(user(id, "carol"))));
        find_roles_by_ids_fake().setup(|_| vec![]);
        save_user_fake().setup(|user| {
            assert_eq!(user.user_name(), "carol");
            assert_eq!(user.display_name(), "Carol C");
            Ok(())
        });

        let (status, _, body) = send(form(
            "PUT",
            &format!("/{id}"),
            "user_name=carol&display_name=Carol+C",
        ))
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.contains(&format!("id=\"user-{id}\"")), "{body}");
        assert!(body.contains("Updated carol."), "{body}");
    }

    #[tokio::test]
    async fn retargets_a_taken_user_name_to_the_notice() {
        let id = Uuid::new_v4();
        find_user_by_id_fake().setup(move |_| Ok(Some(user(id, "alice"))));
        find_roles_by_ids_fake().setup(|_| vec![]);
        save_user_fake().setup(|_| Err(SaveUserError::UserNameAlreadyExists));

        let (status, headers, body) = send(form(
            "PUT",
            &format!("/{id}"),
            "user_name=carol&display_name=Carol",
        ))
        .await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(headers["HX-Retarget"], "#notice");
        assert_eq!(headers["HX-Reswap"], "outerHTML");
        assert!(body.contains("The user name already exists."), "{body}");
    }
}
