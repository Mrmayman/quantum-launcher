pub mod ms;

#[derive(Debug, Clone)]
pub struct AccountData {
    pub access_token: Option<String>,
    pub uuid: String,
    pub username: String,
    pub refresh_token: String,
    pub needs_refresh: bool,

    pub account_type: AccountType,
}

#[derive(Debug, Clone, Copy)]
pub enum AccountType {
    Microsoft,
    ElyBy,
}

impl AccountData {
    #[must_use]
    pub fn is_elyby(&self) -> bool {
        let account_type = self.account_type;
        matches!(account_type, AccountType::ElyBy)
    }
}
