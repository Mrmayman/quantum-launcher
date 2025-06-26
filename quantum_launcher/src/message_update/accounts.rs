use std::collections::HashMap;

use auth::AccountData;
use iced::Task;
use ql_core::IntoStringError;
use ql_instances::auth;

use crate::{
    config::ConfigAccount,
    state::{
        AccountMessage, Launcher, MenuLoginElyBy, MenuLoginMS, Message, ProgressBar, State,
        NEW_ACCOUNT_NAME, OFFLINE_ACCOUNT_NAME,
    },
};

impl Launcher {
    pub fn update_account(&mut self, msg: AccountMessage) -> Task<Message> {
        match msg {
            AccountMessage::Selected(account) => {
                return self.account_selected(account);
            }
            AccountMessage::Response1 { r: Err(err), .. }
            | AccountMessage::Response2(Err(err))
            | AccountMessage::Response3(Err(err))
            | AccountMessage::RefreshComplete(Err(err)) => {
                self.set_error(err);
            }
            AccountMessage::Response1 {
                r: Ok(code),
                is_from_welcome_screen,
            } => {
                return self.account_response_1(code, is_from_welcome_screen);
            }
            AccountMessage::Response2(Ok(token)) => {
                return self.account_response_2(token);
            }
            AccountMessage::Response3(Ok(data)) => {
                return self.account_response_3(data);
            }
            AccountMessage::LogoutCheck => {
                let username = self.accounts_selected.as_ref().unwrap();
                self.state = State::ConfirmAction {
                    msg1: format!("log out of your account: {username}"),
                    msg2: "You can always log in later".to_owned(),
                    yes: Message::Account(AccountMessage::LogoutConfirm),
                    no: Message::LaunchScreenOpen {
                        message: None,
                        clear_selection: false,
                    },
                }
            }
            AccountMessage::LogoutConfirm => {
                let username = self.accounts_selected.clone().unwrap();
                if let Err(err) = auth::ms::logout(&username) {
                    self.set_error(err);
                }
                if let Some(accounts) = &mut self.config.accounts {
                    accounts.remove(&username);
                }
                self.accounts.remove(&username);
                if let Some(idx) = self
                    .accounts_dropdown
                    .iter()
                    .enumerate()
                    .find_map(|(i, n)| (*n == username).then_some(i))
                {
                    self.accounts_dropdown.remove(idx);
                }
                let selected_account = self
                    .accounts_dropdown
                    .first()
                    .cloned()
                    .unwrap_or_else(|| OFFLINE_ACCOUNT_NAME.to_owned());
                self.accounts_selected = Some(selected_account);

                return self.go_to_launch_screen(Option::<String>::None);
            }
            AccountMessage::RefreshComplete(Ok(data)) => {
                self.accounts.insert(data.username.clone(), data);

                let account_data = if let Some(account) = &self.accounts_selected {
                    if account == NEW_ACCOUNT_NAME || account == OFFLINE_ACCOUNT_NAME {
                        None
                    } else {
                        self.accounts.get(account).cloned()
                    }
                } else {
                    None
                };

                return Task::batch([
                    self.go_to_launch_screen::<String>(None),
                    self.launch_game(account_data),
                ]);
            }

            AccountMessage::OpenMicrosoft {
                is_from_welcome_screen,
            } => {
                self.state = State::GenericMessage("Loading Login...".to_owned());
                return Task::perform(auth::ms::login_1_link(), move |n| {
                    Message::Account(AccountMessage::Response1 {
                        r: n.strerr(),
                        is_from_welcome_screen,
                    })
                });
            }
            AccountMessage::OpenElyBy {
                is_from_welcome_screen,
            } => {
                self.state = State::LoginElyBy(MenuLoginElyBy {
                    username: String::new(),
                    password: String::new(),
                    is_loading: false,
                    status: None,
                    show_password: false,
                    is_from_welcome_screen,
                });
            }

            AccountMessage::ElyByUsernameInput(username) => {
                if let State::LoginElyBy(menu) = &mut self.state {
                    menu.username = username;
                }
            }
            AccountMessage::ElyByPasswordInput(password) => {
                if let State::LoginElyBy(menu) = &mut self.state {
                    menu.password = password;
                }
            }
            AccountMessage::ElyByShowPassword(t) => {
                if let State::LoginElyBy(menu) = &mut self.state {
                    menu.show_password = t;
                }
            }
        }
        Task::none()
    }

    fn account_selected(&mut self, account: String) -> Task<Message> {
        if account == NEW_ACCOUNT_NAME {
            self.state = State::AccountLogin;
        } else {
            self.accounts_selected = Some(account);
        }
        Task::none()
    }

    pub fn account_refresh(&mut self, account: &AccountData) -> Task<Message> {
        let (sender, receiver) = std::sync::mpsc::channel();

        self.state = State::AccountLoginProgress(ProgressBar::with_recv(receiver));

        let username = account.username.clone();
        let refresh_token = account.refresh_token.clone();
        Task::perform(
            auth::ms::login_refresh(username, refresh_token, Some(sender)),
            |n| Message::Account(AccountMessage::RefreshComplete(n.strerr())),
        )
    }

    fn account_response_3(&mut self, data: AccountData) -> Task<Message> {
        self.accounts_dropdown.insert(0, data.username.clone());

        if self.config.accounts.is_none() {
            self.config.accounts = Some(HashMap::new());
        }
        let accounts = self.config.accounts.as_mut().unwrap();
        accounts.insert(
            data.username.clone(),
            ConfigAccount {
                uuid: data.uuid.clone(),
                skin: None,
                account_type: Some("Microsoft".to_owned()),
            },
        );

        self.accounts_selected = Some(data.username.clone());
        self.accounts.insert(data.username.clone(), data);

        self.go_to_launch_screen::<String>(None)
    }

    fn account_response_2(&mut self, token: auth::ms::AuthTokenResponse) -> Task<Message> {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.state = State::AccountLoginProgress(ProgressBar::with_recv(receiver));
        Task::perform(auth::ms::login_3_xbox(token, Some(sender), true), |n| {
            Message::Account(AccountMessage::Response3(n.strerr()))
        })
    }

    fn account_response_1(
        &mut self,
        code: auth::ms::AuthCodeResponse,
        is_from_welcome_screen: bool,
    ) -> Task<Message> {
        let (task, handle) = Task::perform(auth::ms::login_2_wait(code.clone()), |n| {
            Message::Account(AccountMessage::Response2(n.strerr()))
        })
        .abortable();

        self.state = State::LoginMS(MenuLoginMS {
            url: code.verification_uri,
            code: code.user_code,
            is_from_welcome_screen,
            _cancel_handle: handle.abort_on_drop(),
        });

        task
    }
}
