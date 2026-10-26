use std::env::VarError;
use std::sync::LazyLock;

use crate::settings::Config;

pub const REGION: &str = "eu-west-1";
pub const CACHE_KEY: &str = "user-profile";
pub const API_KEY: &str = "<your-key-here>";
pub const UPLOAD_TOKEN: &str = "<paste-your-token-here-Xq7vN2mB4dW>";
pub static SESSION_TOKEN: LazyLock<Result<String, VarError>> =
    LazyLock::new(|| std::env::var("ANTHROPIC_AUTH_TOKEN"));

pub fn refresh_token() -> Result<String, VarError> {
    let refresh_token = std::env::var("ANTHROPIC_REFRESH_TOKEN");
    refresh_token
}

pub fn auth_token(config: &Config) -> String {
    let auth_token = config.credentials.anthropic_auth_token.clone();
    auth_token
}
