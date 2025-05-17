use ql_core::{impl_3_errs_jri, IoError, JsonError, RequestError};
use thiserror::Error;

use crate::store::ModError;

const PACK_ERR_PREFIX: &str = "while installing modpack/mod:\n";

#[derive(Debug, Error)]
pub enum PackError {
    #[error("{PACK_ERR_PREFIX}{0}")]
    Io(#[from] IoError),
    #[error("{PACK_ERR_PREFIX}{0}")]
    Json(#[from] JsonError),
    #[error("{PACK_ERR_PREFIX}{0}")]
    Request(#[from] RequestError),
    #[error("{PACK_ERR_PREFIX}while reading zip:\n{0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("{PACK_ERR_PREFIX}while reading file ({1}) from zip:\n{0}")]
    ZipIoError(std::io::Error, String),
    #[error("This modpack requires loader: {expect}\nbut you have {got} installed.\n\nPlease install {expect} from the Mods menu")]
    Loader { expect: String, got: String },
    #[error("This modpack requires Minecraft {expect}\nbut this instance is Minecraft {got}.\n\nPlease create a {expect} instance.")]
    GameVersion { expect: String, got: String },
    #[error("{PACK_ERR_PREFIX}This modpack doesn't have any mod loaders specified.\nIt may be corrupt, unsupported or invalid.\nPlease report this bug in discord.")]
    NoLoadersSpecified,
    #[error("{PACK_ERR_PREFIX}{0}")]
    Mod(#[from] ModError),
}

impl_3_errs_jri!(PackError, Json, Request, Io);
