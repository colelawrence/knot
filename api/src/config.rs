use std::env;
use std::fmt;

mod value;
use value::Value;

#[derive(Clone)]
pub struct Config {
    pub database_url: Value<String>,
    pub google_oauth_client_id: Value<String>,
    pub google_oauth_client_secret: Value<String>,
    pub http_host: Value<String>,
    pub http_port: Value<u16>,
    pub http_public_url: Value<String>,
    pub redis_url: Value<String>,
    pub s3_access_key_id: Value<String>,
    pub s3_secret_access_key: Value<String>,
    pub s3_bucket_prefix: Value<String>,
    pub pepper_0: Value<String>,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DATABASE_URL={}\n", self.database_url)?;
        write!(
            f,
            "GOOGLE_OAUTH_CLIENT_ID={}\n",
            self.google_oauth_client_id
        )?;
        write!(
            f,
            "GOOGLE_OAUTH_CLIENT_SECRET={}\n",
            self.google_oauth_client_secret
        )?;
        write!(f, "HOST={}\n", self.http_host)?;
        write!(f, "PORT={}\n", self.http_port)?;
        write!(f, "PUBLIC_URL={}\n", self.http_public_url)?;
        write!(f, "REDIS_URL={}\n", self.redis_url)?;
        write!(f, "S3_ACCESS_KEY_ID={}\n", self.s3_access_key_id)?;
        write!(f, "S3_SECRET_ACCESS_KEY={}\n", self.s3_secret_access_key)?;
        write!(f, "S3_BUCKET_PREFIX={}\n", self.s3_bucket_prefix)?;
        write!(f, "PEPPER_0={}\n", self.pepper_0)
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DATABASE_URL={:?}\n", self.database_url)?;
        write!(
            f,
            "GOOGLE_OAUTH_CLIENT_ID={:?}\n",
            self.google_oauth_client_id
        )?;
        write!(
            f,
            "GOOGLE_OAUTH_CLIENT_SECRET={:?}\n",
            self.google_oauth_client_secret
        )?;
        write!(f, "HOST={:?}\n", self.http_host)?;
        write!(f, "PORT={:?}\n", self.http_port)?;
        write!(f, "PUBLIC_URL={:?}\n", self.http_public_url)?;
        write!(f, "REDIS_URL={:?}\n", self.redis_url)?;
        write!(f, "S3_ACCESS_KEY_ID={:?}\n", self.s3_access_key_id)?;
        write!(f, "S3_SECRET_ACCESS_KEY={:?}\n", self.s3_secret_access_key)?;
        write!(f, "S3_BUCKET_PREFIX={:?}\n", self.s3_bucket_prefix)?;
        write!(f, "PEPPER_0={:?}\n", self.pepper_0)
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            database_url: Value::default(String::from("postgres://postgres:@localhost/knot")),
            google_oauth_client_id: Value::default(String::from("")),
            google_oauth_client_secret: Value::default(String::from("")).sensitive(),
            http_host: Value::default(String::from("localhost")),
            http_port: Value::default(8088u16),
            http_public_url: Value::default(String::from("http://localhost:8088")),
            redis_url: Value::default(String::from("127.0.0.1:6379")),
            s3_access_key_id: Value::default(String::from("")),
            s3_secret_access_key: Value::default(String::from("")).sensitive(),
            s3_bucket_prefix: Value::default(String::from("knot")),
            pepper_0: Value::default(String::from("")).sensitive(),
        }
    }
}

/// Helper function take either the environment value or the the default
fn env_or(name: &str, default: &Value<String>) -> Value<String> {
    env::var(name).ok().map(Value::env).unwrap_or_else(|| default.clone())
}

impl Config {
    pub fn with_environment(&self) -> Result<Config, String> {
        let http_port_opt = if let Ok(http_port_s) = env::var("HTTP_PORT") {
            Some(
                http_port_s
                    .parse::<u16>()
                    .map_err(|err| format!("Failed to parse port number:{:?}", err))?,
            )
        } else {
            None
        };

        Ok(Config {
            database_url: env_or("DATABASE_URL", &self.database_url),
            google_oauth_client_id: env_or("GOOGLE_OAUTH_CLIENT_ID", &self.google_oauth_client_id),
            google_oauth_client_secret: env_or(
                "GOOGLE_OAUTH_CLIENT_SECRET",
                &self.google_oauth_client_secret,
            ).sensitive(),
            http_host: env_or("HTTP_HOST", &self.http_host),
            http_port: http_port_opt.map(Value::env).unwrap_or_else(|| self.http_port.clone()),
            http_public_url: env_or("PUBLIC_URL", &self.http_public_url),
            redis_url: env_or("REDIS_URL", &self.redis_url),
            s3_access_key_id: env_or("S3_ACCESS_KEY_ID", &self.s3_access_key_id),
            s3_secret_access_key: env_or("S3_SECRET_ACCESS_KEY", &self.s3_secret_access_key).sensitive(),
            s3_bucket_prefix: env_or("S3_BUCKET_PREFIX", &self.s3_bucket_prefix),
            pepper_0: env_or("PEPPER_0", &self.pepper_0).sensitive(),
        })
    }

    pub fn apply_arguments(
        mut self,
        arguments: &clap::ArgMatches,
    ) -> Result<Config, ArgumentError> {
        if let Some(port_s) = arguments.value_of("PORT") {
            if let Ok(port) = port_s.parse::<u16>() {
                self.http_port = Value::arg(port);
            } else {
                return Err(ArgumentError {
                    argument: "PORT",
                    expected: "an integer between 0-65535",
                });
            }
        }
        if let Some(host) = arguments.value_of("HOST") {
            self.http_host = Value::arg(String::from(host));
        }
        if let Some(public_url) = arguments.value_of("PUBLIC_URL") {
            self.http_public_url = Value::arg(String::from(public_url));
        }

        Ok(self)
    }
}

#[derive(Debug)]
pub struct ArgumentError {
    argument: &'static str,
    expected: &'static str,
}
impl fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} must be {}", self.argument, self.expected)
    }
}
