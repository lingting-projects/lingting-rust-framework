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

    pub fn check(
        &self,
        organizations: Option<&[String]>,
        permissions: Option<&[String]>,
        roles: Option<&[String]>,
    ) -> bool {
        if self.anonymous == Some(true) {
            return true;
        }
        self.check_organization(organizations)
            && self.check_permission(permissions)
            && self.check_role(roles)
            && self.rules.as_ref().is_none_or(|rules| {
                rules
                    .iter()
                    .all(|rule| rule.check(organizations, permissions, roles))
            })
            && self.rules_any.as_ref().is_none_or(|rules| {
                rules
                    .iter()
                    .any(|rule| rule.check(organizations, permissions, roles))
            })
    }

    pub fn check_organization(&self, value: Option<&[String]>) -> bool {
        contains_none(&self.organizations, &self.organizations_any, value)
    }

    pub fn check_permission(&self, value: Option<&[String]>) -> bool {
        contains_none(&self.permissions, &self.permissions_any, value)
    }

    pub fn check_role(&self, value: Option<&[String]>) -> bool {
        contains_none(&self.roles, &self.roles_any, value)
    }
}

fn contains_none(
    all: &Option<Vec<String>>,
    any: &Option<Vec<String>>,
    value: Option<&[String]>,
) -> bool {
    let has = all.is_some() || any.is_some();
    let value = match value {
        Some(v) => v,
        None => {
            return !has;
        }
    };

    if value.is_empty() && has {
        return false;
    }

    contains_all(all, value) && contains_any(any, value)
}

fn contains_all(required: &Option<Vec<String>>, actual: &[String]) -> bool {
    required
        .as_ref()
        .is_none_or(|items| items.iter().all(|item| actual.contains(item)))
}

fn contains_any(required: &Option<Vec<String>>, actual: &[String]) -> bool {
    required
        .as_ref()
        .is_none_or(|items| items.iter().any(|item| actual.contains(item)))
}
