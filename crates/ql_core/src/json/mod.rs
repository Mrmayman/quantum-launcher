pub mod fabric;
pub mod forge;
pub mod instance_config;
mod java_files;
mod java_list;
pub mod manifest;
pub mod optifine;
pub mod version;

pub use fabric::FabricJSON;
pub use instance_config::{InstanceConfigJson, OmniarchiveEntry};
pub use java_files::{JavaFile, JavaFileDownload, JavaFileDownloadDetails, JavaFilesJson};
pub use java_list::{
    JavaInstallListing, JavaInstallListingAvailability, JavaInstallListingManifest,
    JavaInstallListingVersion, JavaList, JavaListJson, JavaVersion,
};
pub use manifest::Manifest;
pub use optifine::{JsonOptifine, OptifineArguments, OptifineLibrary};
pub use version::VersionDetails;
