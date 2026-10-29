use serde::{Deserialize, Serialize};

/// Entidad de dominio: un medio de la biblioteca (película, serie o anime).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub id: i64,
    #[serde(rename = "type")]
    pub r#type: String,
    pub title: String,
    pub sort_title: String,
    pub year: Option<i32>,
    pub folder_path: String,
}
