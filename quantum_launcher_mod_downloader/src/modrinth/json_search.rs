use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Search {
    hits: Vec<SearchEntry>,
    offset: usize,
    limit: usize,
    total_hits: usize,
}

impl Search {
    fn get_search_url(query: SearchQuery) -> String {
        let mut url = format!(
            "https://api.modrinth.com/v2/search?index=relevance&query={}",
            query.name
        );
        let mut filters: Vec<Vec<String>> = Vec::new();

        filters.push(vec!["project_type:mod".to_owned()]);

        if query.loaders.is_empty() {
            filters.push(vec![
                "categories:'forge'".to_owned(),
                "categories:'fabric'".to_owned(),
                "categories:'quilt'".to_owned(),
                "categories:'liteloader'".to_owned(),
                "categories:'modloader'".to_owned(),
                "categories:'rift'".to_owned(),
                "categories:'neoforge'".to_owned(),
            ]);
        } else {
            filters.push(
                query
                    .loaders
                    .iter()
                    .map(|loader| format!("categories:'{}'", loader.to_string()))
                    .collect(),
            );
        }

        if query.open_source {
            filters.push(vec!["open_source:true".to_owned()]);
        }
        if query.client_side {
            filters.push(vec!["client_side:required".to_owned()]);
        }
        if query.server_side {
            filters.push(vec!["server_side:required".to_owned()]);
        }

        filters.push(
            query
                .versions
                .iter()
                .map(|version| format!("versions:{version}"))
                .collect(),
        );

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
        url.push(']');

        url
    }

    pub fn search(query: SearchQuery) -> Result<Self, ()> {
        let url = Search::get_search_url(query);

        todo!()
    }
}

pub struct SearchQuery {
    name: String,
    versions: Vec<String>,
    loaders: Vec<Loader>,
    client_side: bool,
    server_side: bool,
    open_source: bool,
}

pub enum Loader {
    Forge,
    Fabric,
    Quilt,
    Liteloader,
    Modloader,
    Rift,
    Neoforge,
}

impl ToString for Loader {
    fn to_string(&self) -> String {
        match self {
            Loader::Forge => "forge",
            Loader::Fabric => "fabric",
            Loader::Quilt => "quilt",
            Loader::Liteloader => "liteloader",
            Loader::Modloader => "modloader",
            Loader::Rift => "rift",
            Loader::Neoforge => "neoforge",
        }
        .to_owned()
    }
}

#[derive(Serialize, Deserialize)]
pub struct SearchEntry {
    slug: String,
    title: String,
    description: String,
    categories: Vec<String>,
    client_side: String,
    server_side: String,
    project_type: String,
    downloads: usize,
    icon_url: String,
    color: usize,
    thread_id: String,
    monetization_status: String,
    project_id: String,
    author: String,
    display_categories: Vec<String>,
    versions: Vec<String>,
    follows: usize,
    date_created: String,
    date_modified: String,
    latest_version: String,
    license: String,
    gallery: Vec<String>,
    featured_gallery: String,
}
