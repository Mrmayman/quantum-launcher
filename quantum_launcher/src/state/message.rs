use std::{
    collections::HashSet,
    path::PathBuf,
    process::ExitStatus,
    sync::{Arc, Mutex},
};

use iced::widget;
use ql_core::{jarmod::JarMods, ListEntry, ModId, StoreBackendType};
use ql_instances::{AccountData, AuthCodeResponse, AuthTokenResponse, UpdateCheckInfo};
use ql_mod_manager::{
    loaders::fabric::FabricVersionListItem,
    store::{CurseforgeNotAllowed, ImageResult, ModIndex, QueryType, RecommendedMod, SearchResult},
};
use tokio::process::Child;

use super::{LaunchTabId, Res};

#[derive(Debug, Clone)]
pub enum InstallFabricMessage {
    End(Res<bool>),
    VersionSelected(String),
    VersionsLoaded(Res<Vec<FabricVersionListItem>>),
    ButtonClicked,
    ScreenOpen { is_quilt: bool },
}

#[derive(Debug, Clone)]
pub enum CreateInstanceMessage {
    ScreenOpen,
    VersionsLoaded(Res<Vec<ListEntry>>),
    VersionSelected(ListEntry),
    NameInput(String),
    Start,
    End(Res<String>),
    ChangeAssetToggle(bool),
    Cancel,
}

#[derive(Debug, Clone)]
pub enum EditInstanceMessage {
    ConfigSaved(Res),
    JavaOverride(String),
    MemoryChanged(f32),
    LoggingToggle(bool),
    CloseLauncherToggle(bool),
    JavaArgsAdd,
    JavaArgEdit(String, usize),
    JavaArgDelete(usize),
    JavaArgShiftUp(usize),
    JavaArgShiftDown(usize),
    GameArgsAdd,
    GameArgEdit(String, usize),
    GameArgDelete(usize),
    GameArgShiftUp(usize),
    GameArgShiftDown(usize),
    RenameEdit(String),
    RenameApply,
}

#[derive(Debug, Clone)]
pub enum ManageModsMessage {
    ScreenOpen,
    ScreenOpenWithoutUpdate,

    ToggleCheckbox((String, ModId), bool),
    ToggleCheckboxLocal(String, bool),

    DeleteSelected,
    DeleteFinished(Res<Vec<ModId>>),
    LocalDeleteFinished(Res),
    LocalIndexLoaded(HashSet<String>),

    ToggleSelected,
    ToggleFinished(Res),

    UpdateMods,
    UpdateModsFinished(Res),
    UpdateCheckResult(Res<Vec<(ModId, String)>>),
    UpdateCheckToggle(usize, bool),

    SelectAll,
    AddFile,
    AddFileDone(Res<HashSet<CurseforgeNotAllowed>>),
}

#[derive(Debug, Clone)]
pub enum ManageJarModsMessage {
    Open,
    ToggleCheckbox(String, bool),
    DeleteSelected,
    AddFile,
    ToggleSelected,
    SelectAll,
    AutosaveFinished((Res, JarMods)),
    MoveUp,
    MoveDown,
}

#[derive(Debug, Clone)]
pub enum InstallModsMessage {
    SearchResult(Res<SearchResult>),
    Open,
    SearchInput(String),
    ImageDownloaded(Res<ImageResult>),
    Click(usize),
    BackToMainScreen,
    LoadData(Res<(ModId, String)>),
    Download(usize),
    DownloadComplete(Res<(ModId, HashSet<CurseforgeNotAllowed>)>),
    IndexUpdated(Res<ModIndex>),
    Scrolled(widget::scrollable::Viewport),
    InstallModpack(ModId),

    ChangeBackend(StoreBackendType),
    ChangeQueryType(QueryType),
}

#[derive(Debug, Clone)]
pub enum InstallOptifineMessage {
    ScreenOpen,
    SelectInstallerStart,
    End(Res),
}

#[derive(Debug, Clone)]
pub enum EditPresetsMessage {
    Open,
    TabChange(String),
    ToggleCheckbox((String, ModId), bool),
    ToggleCheckboxLocal(String, bool),
    SelectAll,
    BuildYourOwn,
    BuildYourOwnEnd(Res<Vec<u8>>),
    Load,
    LoadComplete(Res<HashSet<CurseforgeNotAllowed>>),
    RecommendedModCheck(Res<Vec<RecommendedMod>>),
    RecommendedToggle(usize, bool),
    RecommendedDownload,
    RecommendedDownloadEnd(Res<HashSet<CurseforgeNotAllowed>>),
}

#[derive(Debug, Clone)]
pub enum AccountMessage {
    Selected(String),
    Response1(Res<AuthCodeResponse>),
    Response2(Res<AuthTokenResponse>),
    Response3(Res<AccountData>),
    LogoutCheck,
    LogoutConfirm,
    RefreshComplete(Res<AccountData>),
}

#[derive(Debug, Clone)]
pub enum LauncherSettingsMessage {
    Open,
    ThemePicked(String),
    StylePicked(String),
    UiScale(f64),
    UiScaleApply,
    ClearJavaInstalls,
}

#[derive(Debug, Clone)]
pub enum Message {
    #[allow(unused)]
    Nothing,

    WelcomeContinue1,
    WelcomeContinue2,

    Account(AccountMessage),
    CreateInstance(CreateInstanceMessage),
    EditInstance(EditInstanceMessage),
    ManageMods(ManageModsMessage),
    ManageJarMods(ManageJarModsMessage),
    InstallMods(InstallModsMessage),
    InstallOptifine(InstallOptifineMessage),
    InstallFabric(InstallFabricMessage),
    EditPresets(EditPresetsMessage),
    LauncherSettings(LauncherSettingsMessage),

    LaunchInstanceSelected {
        name: String,
        is_server: bool,
    },
    LaunchUsernameSet(String),
    LaunchStart,
    LaunchScreenOpen {
        message: Option<String>,
        clear_selection: bool,
    },
    LaunchEnd(Res<Arc<Mutex<Child>>>),
    LaunchKill,
    LaunchKillEnd(Res),
    LaunchChangeTab(LaunchTabId),

    LaunchScrollSidebar(f32),

    DeleteInstanceMenu,
    DeleteInstance,

    InstallForgeStart {
        is_neoforge: bool,
    },
    InstallForgeEnd(Res),
    InstallPaperStart,
    InstallPaperEnd(Res),

    UninstallLoaderConfirm(Box<Message>, String),
    UninstallLoaderFabricStart,
    UninstallLoaderForgeStart,
    UninstallLoaderOptiFineStart,
    UninstallLoaderPaperStart,
    UninstallLoaderEnd(Res),

    ExportInstanceOpen,
    ExportInstanceToggleItem(usize, bool),
    ExportInstanceStart,
    ExportInstanceFinished(Res<Vec<u8>>),

    CoreErrorCopy,
    CoreErrorCopyLog,
    CoreOpenLink(String),
    CoreOpenPath(PathBuf),
    CoreCopyText(String),
    CoreTick,
    CoreTickConfigSaved(Res),
    CoreListLoaded(Res<(Vec<String>, bool)>),
    CoreOpenChangeLog,
    CoreOpenIntro,
    CoreEvent(iced::Event, iced::event::Status),
    CoreLogCleanComplete(Res),

    CoreLogToggle,
    CoreLogScroll(isize),
    CoreLogScrollAbsolute(isize),

    LaunchLogScroll(isize),
    LaunchLogScrollAbsolute(isize),
    LaunchEndedLog(Res<(ExitStatus, String)>),
    LaunchCopyLog,

    UpdateCheckResult(Res<UpdateCheckInfo>),
    UpdateDownloadStart,
    UpdateDownloadEnd(Res),

    ServerManageOpen {
        selected_server: Option<String>,
        message: Option<String>,
    },
    ServerManageStartServer(String),
    ServerManageStartServerFinish(Res<(Arc<Mutex<Child>>, bool)>),
    ServerManageEndedLog(Res<(ExitStatus, String)>),
    ServerManageKillServer(String),
    ServerManageEditCommand(String, String),
    ServerManageCopyLog,
    ServerManageSubmitCommand(String),

    ServerCreateScreenOpen,
    ServerCreateVersionsLoaded(Res<Vec<ListEntry>>),
    ServerCreateNameInput(String),
    ServerCreateVersionSelected(ListEntry),
    ServerCreateStart,
    ServerCreateEnd(Res<String>),
}
