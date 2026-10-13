use crate::{
    config::Config,
    domain::entity::{login_session::LoginSession, user_session::UserSession},
    session::Session,
};

impl Session for LoginSession {
    const COOKIE_NAME: &'static str = "__Host-login_session";
    const KEY_PREFIX: &'static str = "session:login";

    fn ttl_seconds() -> u64 {
        Config::login_session_ttl()
    }
}

impl Session for UserSession {
    const COOKIE_NAME: &'static str = "__Host-user_session";
    const KEY_PREFIX: &'static str = "session:user";

    fn ttl_seconds() -> u64 {
        Config::user_session_ttl()
    }
}
