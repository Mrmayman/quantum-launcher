use std::{fmt::Display, path::PathBuf};

use image::ImageReader;
use ql_instances::file_utils::{self, RequestError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Search {
    pub hits: Vec<SearchEntry>,
    pub offset: usize,
    pub limit: usize,
    pub total_hits: usize,
}

impl Search {
    fn get_search_url(query: SearchQuery) -> String {
        let mut url = "https://api.modrinth.com/v2/search?index=relevance&limit=100".to_owned();
        if !query.name.is_empty() {
            url.push_str("&query=");
            url.push_str(&query.name);
        }

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

        url
    }

    pub async fn search(query: SearchQuery) -> Result<Self, ModDownloadError> {
        let url = Search::get_search_url(query);
        // println!("{url}");

        let client = reqwest::Client::new();
        let json = file_utils::download_file_to_string(&client, &url).await?;
        let json: Self = serde_json::from_str(&json)?;

        Ok(json)
    }

    pub async fn search_wrapped(query: SearchQuery) -> Result<Self, String> {
        Self::search(query).await.map_err(|err| err.to_string())
    }

    pub async fn download_icon(
        url: String,
        path: PathBuf,
        name_with_extension: String,
        name: String,
    ) -> Option<(String, String)> {
        let client = reqwest::Client::new();
        // println!("Downloading icon {name_with_extension}");
        let icon = file_utils::download_file_to_bytes(&client, &url)
            .await
            .ok()?;
        let img = ImageReader::new(std::io::Cursor::new(icon))
            .with_guessed_format()
            .ok()?
            .decode()
            .ok()?;

        let img = img.resize(32, 32, image::imageops::FilterType::Nearest);

        img.save(&path).ok()?;

        Some((name_with_extension, name))
    }
}

pub enum ModDownloadError {
    RequestError(RequestError),
    Serde(serde_json::Error),
}

impl Display for ModDownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not send modrinth request: ")?;
        match self {
            ModDownloadError::RequestError(err) => write!(f, "(request) {err}"),
            ModDownloadError::Serde(err) => write!(f, "(json) {err}"),
        }
    }
}

impl From<RequestError> for ModDownloadError {
    fn from(value: RequestError) -> Self {
        Self::RequestError(value)
    }
}

impl From<serde_json::Error> for ModDownloadError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

pub struct SearchQuery {
    pub name: String,
    pub versions: Vec<String>,
    pub loaders: Vec<Loader>,
    pub client_side: bool,
    pub server_side: bool,
    pub open_source: bool,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchEntry {
    pub project_id: String,
    pub project_type: String,
    pub slug: String,
    pub author: String,
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    pub display_categories: Vec<String>,
    pub versions: Vec<String>,
    pub downloads: usize,
    pub follows: usize,
    pub icon_url: String,
    pub date_created: String,
    pub date_modified: String,
    pub latest_version: String,
    pub license: String,
    pub client_side: String,
    pub server_side: String,
    pub gallery: Vec<String>,
    pub featured_gallery: Option<String>,
    pub color: Option<usize>,
    pub thread_id: Option<String>,
    pub monetization_status: Option<String>,
}
