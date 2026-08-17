use crate::types::{R, RCodeKind, R_CODE_SUCCCESS, R_MESSAGE_SUCCCESS};
use framework_proc_auto::auto_enum_impl;

#[auto_enum_impl]
impl RCodeKind {
    #[auto_enum_field]
    pub fn code(&self) -> u32 {
        match self {
            RCodeKind::Success => R_CODE_SUCCCESS,
            RCodeKind::Parameter => 400,
            RCodeKind::Unauthorized => 401,
            RCodeKind::Forbidden => 403,
            RCodeKind::Internal => 500,
        }
    }
}

impl<D> R<D> {
    pub fn ok(data: D) -> Self {
        Self {
            code: R_CODE_SUCCCESS,
            message: R_MESSAGE_SUCCCESS.to_string(),
            data: Some(data),
        }
    }
    pub fn ok_none() -> Self {
        Self {
            code: R_CODE_SUCCCESS,
            message: R_MESSAGE_SUCCCESS.to_string(),
            data: None,
        }
    }

    pub fn failed<M>(message: M) -> Self
    where
        M: ToString,
    {
        Self {
            code: 500,
            message: message.to_string(),
            data: None,
        }
    }
}

impl<D, E> From<Result<D, E>> for R<D>
where
    E: ToString,
{
    fn from(result: Result<D, E>) -> Self {
        match result {
            Ok(data) => Self {
                code: R_CODE_SUCCCESS,
                message: R_MESSAGE_SUCCCESS.to_string(),
                data: Some(data),
            },
            Err(error) => Self {
                code: 500,
                message: error.to_string(),
                data: None,
            },
        }
    }
}
