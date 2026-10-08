pub mod peek;
pub mod save;
pub mod take;

fn key(request_uri: &str) -> String {
    format!("par:{request_uri}")
}
