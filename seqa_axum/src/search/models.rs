use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct SearchRequest {
    pub coordinates: String,
    pub path: String,
}
