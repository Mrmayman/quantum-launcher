use std::{
    fmt::Display,
    sync::{mpsc::Sender, Arc},
};

use omniarchive_api::{ListEntry, MinecraftVersionCategory, WebScrapeError};
use ql_core::{err, json::manifest::Manifest, JsonDownloadError};

pub enum ListError {
    JsonDownloadError(JsonDownloadError),
    WebScrapeError(WebScrapeError),
}

impl From<JsonDownloadError> for ListError {
    fn from(error: JsonDownloadError) -> Self {
        ListError::JsonDownloadError(error)
    }
}

impl From<WebScrapeError> for ListError {
    fn from(error: WebScrapeError) -> Self {
        ListError::WebScrapeError(error)
    }
}

impl Display for ListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not list versions: ")?;
        match self {
            ListError::JsonDownloadError(err) => write!(f, "{err}"),
            ListError::WebScrapeError(err) => write!(f, "{err}"),
        }
    }
}

async fn list(sender: Option<Arc<Sender<()>>>) -> Result<Vec<ListEntry>, ListError> {
    let manifest = Manifest::download().await?;
    let mut version_list: Vec<ListEntry> = manifest
        .versions
        .iter()
        .filter_map(|n| {
            (n.r#type == "release" || n.r#type == "snapshot")
                .then_some(ListEntry::Normal(n.id.clone()))
        })
        .collect();

    if let Err(err) = add_omniarchive_versions(&mut version_list, sender).await {
        err!("error getting omniarchive version list: {err}");
        version_list.extend(manifest.versions.iter().filter_map(|n| {
            (!(n.r#type == "release" || n.r#type == "snapshot"))
                .then_some(ListEntry::Normal(n.id.clone()))
        }));
    }

    Ok(version_list)
}

async fn add_omniarchive_versions(
    normal_list: &mut Vec<ListEntry>,
    progress: Option<Arc<Sender<()>>>,
) -> Result<(), ListError> {
    for category in MinecraftVersionCategory::all_client().into_iter().rev() {
        let versions = category.download_index(progress.clone(), false).await?;
        for url in versions.into_iter().rev() {
            let name = if let Some(name) = url
                .strip_prefix("https://vault.omniarchive.uk/archive/java/client-")
                .and_then(|n| n.strip_suffix(".jar"))
            {
                name.to_owned()
            } else {
                url.clone()
            };
            normal_list.push(ListEntry::Omniarchive {
                category: category.clone(),
                name,
                url,
            });
        }
    }
    Ok(())
}

/// Returns a list of all available versions of the game.
pub async fn list_versions(sender: Option<Arc<Sender<()>>>) -> Result<Vec<ListEntry>, String> {
    list(sender).await.map_err(|n| n.to_string())
}
