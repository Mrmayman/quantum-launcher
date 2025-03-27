use serde::Deserialize;

use crate::mod_manager::SearchQuery;

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

pub fn get_url(query: &SearchQuery) -> String {
    let mut url = "https://api.modrinth.com/v2/search?index=relevance&limit=100".to_owned();
    if !query.name.is_empty() {
        url.push_str("&query=");
        url.push_str(&query.name);
    }

    let mut filters: Vec<Vec<String>> = Vec::new();

    filters.push(vec!["project_type:mod".to_owned()]);

    // if query.loaders.is_empty() {
    //     filters.push(vec![
    //         "categories:'forge'".to_owned(),
    //         "categories:'fabric'".to_owned(),
    //         "categories:'quilt'".to_owned(),
    //         "categories:'liteloader'".to_owned(),
    //         "categories:'modloader'".to_owned(),
    //         "categories:'rift'".to_owned(),
    //         "categories:'neoforge'".to_owned(),
    //     ]);
    // }

    filters.push(vec![format!(
        "categories:'{}'",
        query.loader.to_modrinth_str()
    )]);

    // if query.open_source {
    //     filters.push(vec!["open_source:true".to_owned()]);
    // }
    // if query.client_side {
    //     filters.push(vec!["client_side:required".to_owned()]);
    // }
    if query.server_side {
        filters.push(vec!["server_side:required".to_owned()]);
    }

    filters.push(vec![format!("versions:{}", query.version)]);

    url.push_str("&facets=[");

    let num_filters = filters.len();
    for (idx, filter) in filters.iter().enumerate() {
        if !filter.is_empty() {
            url.push('[');
        }

        let num_subfilters = filter.len();
        for (sub_idx, subfilter) in filter.iter().enumerate() {
            url.push_str(&format!("\"{subfilter}\""));
            url.push(if sub_idx + 1 < num_subfilters {
                ','
            } else {
                ']'
            });
        }

        url.push(if idx + 1 < num_filters { ',' } else { ']' });
    }

    url
}
