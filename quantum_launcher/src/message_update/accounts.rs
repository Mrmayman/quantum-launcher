use std::collections::HashMap;

use iced::Task;
use ql_core::IntoStringError;

use crate::{
    config::ConfigAccount,
    launcher_state::{
        AccountMessage, Launcher, Message, ProgressBar, State, NEW_ACCOUNT_NAME,
        OFFLINE_ACCOUNT_NAME,
    },
};

impl Launcher {
    pub fn update_account(&mut self, msg: AccountMessage) -> Task<Message> {
        match msg {
            AccountMessage::Selected(account) => {
                return self.account_selected(account);
            }
            AccountMessage::Response1(Err(err))
            | AccountMessage::Response2(Err(err))
            | AccountMessage::Response3(Err(err))
            | AccountMessage::RefreshComplete(Err(err)) => {
                self.set_error(err);
            }
            AccountMessage::Response1(Ok(code)) => {
                return self.account_response_1(code);
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
                if let Err(err) = ql_instances::logout(&username) {
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
        }
        Task::none()
    }

    fn account_selected(&mut self, account: String) -> Task<Message> {
        if account == NEW_ACCOUNT_NAME {
            self.state = State::GenericMessage("Loading Login...".to_owned());
            Task::perform(ql_instances::login_1_link(), |n| {
                Message::Account(AccountMessage::Response1(n.strerr()))
            })
        } else {
            self.accounts_selected = Some(account);
            Task::none()
        }
    }

    pub fn account_refresh(&mut self, account: &ql_instances::AccountData) -> Task<Message> {
        let (sender, receiver) = std::sync::mpsc::channel();

        self.state = State::AccountLoginProgress(ProgressBar::with_recv(receiver));

        let username = account.username.clone();
        let refresh_token = account.refresh_token.clone();
        Task::perform(
            ql_instances::login_refresh(username, refresh_token, Some(sender)),
            |n| Message::Account(AccountMessage::RefreshComplete(n.strerr())),
        )
    }

    fn account_response_3(&mut self, data: ql_instances::AccountData) -> Task<Message> {
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
            },
        );

        self.accounts_selected = Some(data.username.clone());
        self.accounts.insert(data.username.clone(), data);

        self.go_to_launch_screen::<String>(None)
    }

    fn account_response_2(&mut self, token: ql_instances::AuthTokenResponse) -> Task<Message> {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.state = State::AccountLoginProgress(ProgressBar::with_recv(receiver));
        Task::perform(ql_instances::login_3_xbox(token, Some(sender)), |n| {
            Message::Account(AccountMessage::Response3(n.strerr()))
        })
    }

    fn account_response_1(&mut self, code: ql_instances::AuthCodeResponse) -> Task<Message> {
        // I have no idea how many rustaceans will
        // yell at me after they see this. (WTF: )
        let code2 = code.clone();
        let (task, handle) = Task::perform(ql_instances::login_2_wait(code2), |n| {
            Message::Account(AccountMessage::Response2(n.strerr()))
        })
        .abortable();
        self.state = State::AccountLogin {
            url: code.verification_uri,
            code: code.user_code,
            _cancel_handle: handle.abort_on_drop(),
        };
        task
    }
}
