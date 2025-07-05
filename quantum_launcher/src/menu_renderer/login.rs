use iced::widget;

use crate::{
    icon_manager,
    state::{AccountMessage, MenuLoginElyBy, MenuLoginMS, Message, NEW_ACCOUNT_NAME},
};

use super::{back_button, button_with_icon, Element};

impl MenuLoginElyBy {
    pub fn view(&self, tick_timer: usize) -> Element {
        let status: Element = if self.is_loading {
            let dots = ".".repeat((tick_timer % 3) + 1);
            widget::text!("Loading{dots}").into()
        } else {
            button_with_icon(icon_manager::tick(), "Login", 16)
                .on_press(Message::Account(AccountMessage::ElyByLogin))
                .into()
        };

        let padding = iced::Padding {
            top: 5.0,
            bottom: 5.0,
            right: 10.0,
            left: 10.0,
        };

        let password_input = widget::text_input("Enter Password...", &self.password)
            .padding(padding)
            .on_input(|n| Message::Account(AccountMessage::ElyByPasswordInput(n)));
        let password_input = if self.password.is_empty() || self.show_password {
            password_input
        } else {
            password_input.font(iced::Font::with_name("Password Asterisks"))
        };

        widget::column![
            back_button().on_press(if self.is_from_welcome_screen {
                Message::WelcomeContinueToAuth
            } else {
                Message::Account(AccountMessage::Selected(NEW_ACCOUNT_NAME.to_owned()))
            }),
            widget::row![
                widget::horizontal_space(),
                widget::column![
                    widget::vertical_space(),
                    widget::text("Username/Email:").size(12),
                    widget::text_input("Enter Username/Email...", &self.username)
                        .padding(padding)
                        .on_input(|n| Message::Account(AccountMessage::ElyByUsernameInput(n))),
                    widget::text("Password:").size(12),
                    password_input,
                    widget::checkbox("Show Password", self.show_password)
                        .size(14)
                        .text_size(14)
                        .on_toggle(|t| Message::Account(AccountMessage::ElyByShowPassword(t))),
                    widget::Column::new().push_maybe(self.otp.as_deref().map(|otp| {
                        widget::column![
                            widget::text("OTP:").size(12),
                            widget::text_input("Enter Username/Email...", otp)
                                .padding(padding)
                                .on_input(|n| Message::Account(AccountMessage::ElyByOtpInput(n))),
                        ]
                        .spacing(5)
                    })),
                    status,
                    widget::Space::with_height(5),
                    widget::row![
                        widget::text("Or").size(14),
                        widget::button(widget::text("Create an account").size(14)).on_press(
                            Message::CoreOpenLink("https://account.ely.by/register".to_owned())
                        )
                    ]
                    .align_y(iced::Alignment::Center)
                    .spacing(5)
                    .wrap(),
                    widget::vertical_space(),
                ]
                .align_x(iced::Alignment::Center)
                .spacing(5),
                widget::horizontal_space(),
            ]
        ]
        .padding(10)
        .into()
    }
}

impl MenuLoginMS {
    pub fn view<'a>(&self) -> Element<'a> {
        widget::column![
            back_button().on_press(if self.is_from_welcome_screen {
                Message::WelcomeContinueToAuth
            } else {
                Message::Account(AccountMessage::Selected(NEW_ACCOUNT_NAME.to_owned()))
            }),
            widget::row!(
                widget::horizontal_space(),
                widget::column!(
                    widget::vertical_space(),
                    widget::text("Login to Microsoft").size(20),
                    "Open this link and enter the code:",
                    widget::text!("Code: {}", self.code),
                    widget::button("Copy").on_press(Message::CoreCopyText(self.code.clone())),
                    widget::text!("Link: {}", self.url),
                    widget::button("Open").on_press(Message::CoreOpenLink(self.url.clone())),
                    widget::vertical_space(),
                )
                .spacing(5)
                .align_x(iced::Alignment::Center),
                widget::horizontal_space()
            )
        ]
        .padding(10)
        .into()
    }
}
