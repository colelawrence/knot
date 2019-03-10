#[derive(Serialize, Deserialize)]
pub struct UserSession {
    /// User's state key for associating login with session
    #[serde(rename = "k")]
    pub key: String,
    #[serde(rename = "ri")]
    pub user_token_resource_id: Option<String>,
    #[serde(rename = "ui")]
    pub user_id: Option<String>,
}
