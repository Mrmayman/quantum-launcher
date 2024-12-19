use std::{collections::HashSet, rc::Rc};

use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::Node;
use ql_core::file_utils;

mod error;
pub use error::WebScrapeError;

#[derive(Clone, Debug)]
pub enum MinecraftVersionCategory {
    PreClassic,
    Classic,
    Indev,
    Infdev,
    Alpha,
    Beta,
}

impl MinecraftVersionCategory {
    pub fn all() -> Vec<MinecraftVersionCategory> {
        vec![
            MinecraftVersionCategory::PreClassic,
            MinecraftVersionCategory::Classic,
            MinecraftVersionCategory::Indev,
            MinecraftVersionCategory::Infdev,
            MinecraftVersionCategory::Alpha,
            MinecraftVersionCategory::Beta,
        ]
    }

    fn get_url(&self) -> String {
        format!(
            "https://vault.omniarchive.uk/archive/java/client-{}/index.html",
            match self {
                MinecraftVersionCategory::PreClassic => "preclassic",
                MinecraftVersionCategory::Classic => "classic",
                MinecraftVersionCategory::Indev => "indev",
                MinecraftVersionCategory::Infdev => "infdev",
                MinecraftVersionCategory::Alpha => "alpha",
                MinecraftVersionCategory::Beta => "beta",
            }
        )
    }

    pub async fn download_index(&self) -> Result<Vec<String>, WebScrapeError> {
        let url = self.get_url();

        let client = reqwest::Client::new();

        let mut buffer = Vec::new();
        let mut deeper_buffer = Vec::new();
        let mut visited = HashSet::new();

        let mut links = scrape_links(&client, &url, &mut buffer, &mut visited).await?;

        while !buffer.is_empty() || !deeper_buffer.is_empty() {
            for link in &buffer {
                let scraped_links =
                    scrape_links(&client, &link, &mut deeper_buffer, &mut visited).await?;
                links.extend_from_slice(&scraped_links);
            }
            buffer.clear();
            for link in &deeper_buffer {
                let scraped_links = scrape_links(&client, &link, &mut buffer, &mut visited).await?;
                links.extend_from_slice(&scraped_links);
            }
            deeper_buffer.clear();
        }
        Ok(links)
    }
}

async fn scrape_links(
    client: &reqwest::Client,
    url: &str,
    deeper_buffer: &mut Vec<String>,
    visited: &mut HashSet<String>,
) -> Result<Vec<String>, WebScrapeError> {
    if !visited.insert(url.to_owned()) {
        return Ok(Vec::new());
    }

    let file = file_utils::download_file_to_string(client, url, true).await?;
    let dom = html5ever::parse_document(
        markup5ever_rcdom::RcDom::default(),
        html5ever::ParseOpts::default(),
    )
    .from_utf8()
    .read_from(&mut file.as_bytes())
    .unwrap();
    let e_html = find_elem(&dom.document, "html")?;
    let e_body = find_elem(&e_html, "body")?;
    let e_code = find_elem(&e_body, "code")?;

    let mut links = Vec::new();
    for n in e_code.children.borrow().iter().skip(1) {
        if let markup5ever_rcdom::NodeData::Element { name, attrs, .. } = &n.data {
            if name.local.to_string() != "a" {
                continue;
            }
            if let Some(a) = attrs
                .borrow()
                .iter()
                .find(|a| a.name.local.to_string() == "href")
            {
                let link = a.value.to_string();
                if link.ends_with("index.html") {
                    if link != "https://vault.omniarchive.uk/archive/java/index.html" {
                        deeper_buffer.push(link);
                    }
                } else if !link.ends_with(".exe") {
                    links.push(link);
                }
            }
        }
    }

    Ok(links)
}

fn find_elem(dom: &Node, element_name: &str) -> Result<Rc<Node>, WebScrapeError> {
    dom.children
        .borrow()
        .iter()
        .cloned()
        .find(|n| match &n.data {
            markup5ever_rcdom::NodeData::Element { name, .. } => {
                name.local.to_string() == element_name
            }
            _ => false,
        })
        .ok_or(WebScrapeError::ElementNotFound(element_name.to_owned()))
}
