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
            loader => {
                err!("Unknown loader: {loader}");
                Err(())
            }
        }
    }
}

impl Loader {
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
}
