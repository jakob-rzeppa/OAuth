use rand::distr::{Alphanumeric, SampleString};

use crate::config::CONFIG;

#[fnmock::fakeable]
pub fn generate_temporary_password() -> String {
    let temporary_password: String =
        Alphanumeric.sample_string(&mut rand::rng(), CONFIG.temporary_password_length());

    temporary_password
}
