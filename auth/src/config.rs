use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub google_oauth_client_id: String,
    pub google_oauth_client_secret: String,
    pub http_allowed_origins: String,
    pub http_bind_address: String,
    pub http_public_url: String,
    pub redis_url: String,
    pub pepper_0: String,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            database_url: String::from("postgres://postgres:@localhost/app"),
            google_oauth_client_id: String::from(""),
            google_oauth_client_secret: String::from(""),
            http_allowed_origins: String::from(""),
            http_bind_address: String::from("127.0.0.1:8088"),
            http_public_url: String::from("http://127.0.0.1:8088"),
            redis_url: String::from("127.0.0.1:6379"),
            pepper_0: String::from(""),
        }
    }
}

pub trait NotEmpty
where
    Self: Sized,
{
    fn not_empty(&self) -> Option<Self>;
}

impl NotEmpty for String {
    fn not_empty(&self) -> Option<Self> {
        if self.len() == 0 {
            None
        } else {
            Some(self.to_string())
        }
    }
}

/// Helper function take either the environment value or the the default
fn env_or(name: &str, default: &str) -> String {
    env::var(name).ok().unwrap_or_else(|| default.to_string())
}

impl Config {
    pub fn with_environment(&self) -> Config {
        Config {
            database_url: env_or("DATABASE_URL", &self.database_url),
            google_oauth_client_id: env_or("GOOGLE_OAUTH_CLIENT_ID", &self.google_oauth_client_id),
            google_oauth_client_secret: env_or(
                "GOOGLE_OAUTH_CLIENT_SECRET",
                &self.google_oauth_client_secret,
            ),
            http_bind_address: env_or("HTTP_BIND_ADDRESS", &self.http_bind_address),
            http_public_url: env_or("PUBLIC_URL", &self.http_public_url),
            http_allowed_origins: env_or("HTTP_ALLOWED_ORIGINS", &self.http_allowed_origins),
            redis_url: env_or("REDIS_URL", &self.redis_url),
            pepper_0: env_or("PEPPER_0", &self.pepper_0),
        }
    }
}
