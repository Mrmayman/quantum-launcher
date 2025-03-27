use std::collections::HashMap;

use ql_core::JsonDownloadError;
use serde::Deserialize;

use crate::mod_manager::Query;

pub async fn do_request(query: &Query) -> Result<Search, JsonDownloadError> {
    const SEARCH_URL: &str = "https://api.modrinth.com/v2/search";

    let mut params = HashMap::from([
        ("index", "relevance".to_owned()),
        ("limit", "200".to_owned()),
    ]);
    if !query.name.is_empty() {
        params.insert("query", query.name.clone());
    }

    let mut filters = vec![
        vec!["project_type:mod".to_owned()],
        vec![format!("categories:'{}'", query.loader.to_modrinth_str())],
        vec![format!("versions:{}", query.version)],
    ];
    if query.server_side {
        filters.push(vec![format!("versions:{}", query.version)]);
    }

    let filters = serde_json::to_string(&filters)?;
    params.insert("facets", filters);

    let text = ql_core::CLIENT
        .get(SEARCH_URL)
        .query(&params)
        .send()
        .await?
        .text()
        .await?;

    let json: Search = serde_json::from_str(&text)?;

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
    // pub project_type: String,
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
