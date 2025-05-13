use std::collections::HashMap;

use ql_core::{IntoJsonError, JsonDownloadError};
use serde::Deserialize;

use crate::store::{Query, QueryType};

pub async fn do_request(
    query: &Query,
    offset: usize,
    query_type: QueryType,
) -> Result<Search, JsonDownloadError> {
    const SEARCH_URL: &str = "https://api.modrinth.com/v2/search";

    let mut params = HashMap::from([
        ("index", "relevance".to_owned()),
        ("limit", "100".to_owned()),
        ("offset", offset.to_string()),
    ]);
    if !query.name.is_empty() {
        params.insert("query", query.name.clone());
    }

    let mut filters = vec![
        vec![format!("project_type:{}", query_type.to_modrinth_str())],
        vec![format!("versions:{}", query.version)],
    ];

    if let QueryType::Mods = query_type {
        if let Some(loader) = query.loader {
            filters.push(vec![format!("categories:'{}'", loader.to_modrinth_str())])
        }
    }

    let filters = serde_json::to_string(&filters).json_to()?;
    params.insert("facets", filters);

    let text = ql_core::CLIENT
        .get(SEARCH_URL)
        .query(&params)
        .send()
        .await?
        .text()
        .await?;

    let json: Search = serde_json::from_str(&text).json(text)?;

    Ok(json)
}

#[derive(Deserialize, Debug, Clone)]
pub struct Search {
    pub hits: Vec<Entry>,
    // pub offset: usize,
    // pub limit: usize,
    // pub total_hits: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Entry {
    pub title: String,
    pub project_id: String,
    pub icon_url: String,
    pub description: String,
    pub downloads: usize,
    pub slug: String,
    pub project_type: String,
    // pub author: String,
    // pub categories: Vec<String>,
    // pub display_categories: Vec<String>,
    // pub versions: Vec<String>,
    // pub follows: usize,
    // pub date_created: String,
    // pub date_modified: String,
    // pub latest_version: String,
    // pub license: String,
    // pub client_side: String,
    // pub server_side: String,
    // pub gallery: Vec<String>,
    // pub featured_gallery: Option<String>,
    // pub color: Option<usize>,
    // pub thread_id: Option<String>,
    // pub monetization_status: Option<String>,
}
