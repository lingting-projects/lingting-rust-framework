use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthRule {
    pub anonymous: Option<bool>,
    pub organizations: Option<Vec<String>>,
    pub organizations_any: Option<Vec<String>>,
    pub permissions: Option<Vec<String>>,
    pub permissions_any: Option<Vec<String>>,
    pub roles: Option<Vec<String>>,
    pub roles_any: Option<Vec<String>>,
    pub rules: Option<Vec<AuthRule>>,
    pub rules_any: Option<Vec<AuthRule>>,
}

impl AuthRule {
    pub fn anonymous() -> Self {
        Self {
            anonymous: Some(true),
            ..Default::default()
        }
    }

    pub fn login() -> Self {
        Self {
            anonymous: Some(false),
            ..Default::default()
        }
    }

    pub fn admin() -> Self {
        Self {
            roles: Some(vec!["relayx".to_string()]),
            ..Default::default()
        }
    }
}
