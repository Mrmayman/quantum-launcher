use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use serde::Deserialize;

use crate::store::ModError;

use super::{get_mc_id, send_request};

#[derive(Deserialize, Clone, Debug)]
pub struct Categories {
    pub data: Vec<Category>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Category {
    pub id: i32,
    pub slug: String,
}

pub static CATEGORIES: LazyLock<Mutex<Option<Categories>>> = LazyLock::new(|| Mutex::new(None));

pub async fn get_categories() -> Result<Categories, ModError> {
    // Can't just lock it once because of async thread safety issues
    let is_none = CATEGORIES.lock().unwrap().is_none();
    if is_none {
        let mc_id = get_mc_id().await?;
        let params = HashMap::from([("gameId", mc_id.to_string())]);
        let categories = send_request("categories", &params).await?;
        let categories: Categories = serde_json::from_str(&categories)?;

        *CATEGORIES.lock().unwrap() = Some(categories);
    }
    Ok(CATEGORIES.lock().unwrap().clone().unwrap())
}
