use iced::Task;
use ql_core::{pt, DownloadProgress, InstanceSelection, IntoStringError, ListEntry};

use crate::state::{
    CreateInstanceMessage, Launcher, MenuCreateInstance, Message, ProgressBar, State,
};

impl Launcher {
    pub fn update_create_instance(&mut self, message: CreateInstanceMessage) -> Task<Message> {
        match message {
            CreateInstanceMessage::ScreenOpen => return self.go_to_create_screen(),
            CreateInstanceMessage::VersionsLoaded(result) => {
                self.create_instance_finish_loading_versions_list(result);
            }
            CreateInstanceMessage::VersionSelected(selected_version) => {
                self.select_created_instance_version(selected_version);
            }
            CreateInstanceMessage::NameInput(name) => self.update_created_instance_name(name),
            CreateInstanceMessage::Start => return self.create_instance(),
            CreateInstanceMessage::End(result) => match result {
                Ok(instance) => {
                    self.selected_instance = Some(InstanceSelection::Instance(instance));
                    return self.go_to_launch_screen(Some("Created Instance".to_owned()));
                }
                Err(n) => self.set_error(n),
            },
            CreateInstanceMessage::ChangeAssetToggle(t) => {
                if let State::Create(MenuCreateInstance::Choosing {
                    download_assets, ..
                }) = &mut self.state
                {
                    *download_assets = t;
                }
            }
            CreateInstanceMessage::Cancel => {
                return self.go_to_launch_screen(Option::<String>::None)
            }
            CreateInstanceMessage::Import => {
                if let Some(file) = rfd::FileDialog::new()
                    .set_title("Select an instance...")
                    .pick_file()
                {
                    let (send, recv) = std::sync::mpsc::channel();
                    let progress = ProgressBar::with_recv(recv);

                    // I know this doesn't look necessary but there's
                    // a weird untrackable bug where importing instance
                    // screen just doesn't appear, and the Task runs
                    // silently in the background.
                    //
                    // I hope I manage to fix it in the future.
                    pt!("(Internal): Setting state to ImportingInstance...");

                    self.state = State::Create(MenuCreateInstance::ImportingInstance(progress));

                    return Task::perform(
                        ql_packager::import_instance(file.clone(), true, Some(send)),
                        |n| {
                            Message::CreateInstance(CreateInstanceMessage::ImportResult(n.strerr()))
                        },
                    );
                }
            }
            CreateInstanceMessage::ImportResult(res) => {
                match res {
                    Ok(instance) => {
                        let is_valid_modpack = instance.is_some();
                        self.selected_instance = instance;
                        if is_valid_modpack {
                            return self.go_to_launch_screen(None::<String>);
                        } else {
                            self.set_error(r#"the file you imported isn't a valid QuantumLauncher/MultiMC instance.

If you meant to import a Modrinth/Curseforge/Preset pack,
create a instance with the matching version,
then go to "Mods->Add File""#);
                        }
                    }
                    Err(err) => self.set_error(err),
                }
            }
        }
        Task::none()
    }

    fn create_instance_finish_loading_versions_list(
        &mut self,
        result: Result<Vec<ListEntry>, String>,
    ) {
        match result {
            Ok(versions) => {
                self.client_version_list_cache = Some(versions.clone());
                let combo_state = iced::widget::combo_box::State::new(versions.clone());
                self.state = State::Create(MenuCreateInstance::Choosing {
                    instance_name: String::new(),
                    selected_version: None,
                    download_assets: true,
                    combo_state: Box::new(combo_state),
                });
            }
            Err(n) => self.set_error(n),
        }
    }

    fn go_to_create_screen(&mut self) -> Task<Message> {
        if let Some(versions) = self.client_version_list_cache.clone() {
            let combo_state = iced::widget::combo_box::State::new(versions.clone());
            self.state = State::Create(MenuCreateInstance::Choosing {
                instance_name: String::new(),
                selected_version: None,
                download_assets: true,
                combo_state: Box::new(combo_state),
            });
            Task::none()
        } else {
            let (task, handle) = Task::perform(ql_instances::list_versions(), |n| {
                Message::CreateInstance(CreateInstanceMessage::VersionsLoaded(n.strerr()))
            })
            .abortable();

            self.state = State::Create(MenuCreateInstance::LoadingList {
                _handle: handle.abort_on_drop(),
            });

            task
        }
    }

    fn select_created_instance_version(&mut self, entry: ListEntry) {
        if let State::Create(MenuCreateInstance::Choosing {
            selected_version, ..
        }) = &mut self.state
        {
            *selected_version = Some(entry);
        }
    }

    fn update_created_instance_name(&mut self, name: String) {
        if let State::Create(MenuCreateInstance::Choosing { instance_name, .. }) = &mut self.state {
            *instance_name = name;
        }
    }

    fn create_instance(&mut self) -> Task<Message> {
        if let State::Create(MenuCreateInstance::Choosing {
            instance_name,
            download_assets,
            selected_version,
            ..
        }) = &mut self.state
        {
            let (sender, receiver) = std::sync::mpsc::channel::<DownloadProgress>();
            let progress = ProgressBar {
                num: 0.0,
                message: Some("Started download".to_owned()),
                receiver,
                progress: DownloadProgress::DownloadingJsonManifest,
            };

            let instance_name = instance_name.clone();
            let version = selected_version.clone().unwrap();
            let download_assets = *download_assets;

            self.state = State::Create(MenuCreateInstance::DownloadingInstance(progress));

            return Task::perform(
                ql_instances::create_instance(
                    instance_name.clone(),
                    version,
                    Some(sender),
                    download_assets,
                ),
                |n| Message::CreateInstance(CreateInstanceMessage::End(n.strerr())),
            );
        }
        Task::none()
    }
}
