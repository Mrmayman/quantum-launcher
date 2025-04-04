use crate::err;

#[derive(Debug, Clone, Copy)]
pub enum Loader {
    Forge,
    Fabric,
    Quilt,

    // The launcher supports these, but modrinth doesn't
    // (so no Mod Store):
    OptiFine,
    Paper,

    // The launcher doesn't currently support these:
    Neoforge,
    Liteloader,
    Modloader,
    Rift,
}

impl TryFrom<&str> for Loader {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Forge" => Ok(Loader::Forge),
            "Fabric" => Ok(Loader::Fabric),
            "Quilt" => Ok(Loader::Quilt),
            "OptiFine" => Ok(Loader::OptiFine),
            "Paper" => Ok(Loader::Paper),
            "NeoForge" => Ok(Loader::Neoforge),
            loader => {
                err!("Unknown loader: {loader}");
                Err(())
            }
        }
    }
}

impl Loader {
    #[must_use]
    pub fn to_modrinth_str(self) -> &'static str {
        match self {
            Loader::Forge => "forge",
            Loader::Fabric => "fabric",
            Loader::Quilt => "quilt",
            Loader::Liteloader => "liteloader",
            Loader::Modloader => "modloader",
            Loader::Rift => "rift",
            Loader::Neoforge => "neoforge",
            Loader::OptiFine => "optifine",
            Loader::Paper => "paper",
        }
    }

    #[must_use]
    pub fn to_curseforge(&self) -> &'static str {
        match self {
            Loader::Forge => "1",
            Loader::Fabric => "4",
            Loader::Quilt => "5",
            Loader::Neoforge => "6",
            Loader::Liteloader => "3",
            Loader::Rift | Loader::Paper | Loader::Modloader | Loader::OptiFine => {
                err!("Unsupported loader for curseforge: {self:?}");
                "0"
            } // Not supported
        }
    }

    #[must_use]
    pub fn to_curseforge_str(&self) -> Option<&'static str> {
        match self {
            Loader::Forge => Some("Forge"),
            Loader::Fabric => Some("Fabric"),
            Loader::Quilt => Some("Quilt"),
            Loader::Neoforge => Some("NeoForge"),
            Loader::Liteloader => Some("LiteLoader"),
            Loader::Rift | Loader::Paper | Loader::Modloader | Loader::OptiFine => {
                err!("Unsupported loader for curseforge: {self:?}");
                None
            } // Not supported
        }
    }
}
