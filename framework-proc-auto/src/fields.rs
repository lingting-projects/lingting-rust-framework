use syn::{Field, Token, punctuated::Punctuated};

use crate::attributes::{ensure_serde_adapter, ensure_serde_default, ensure_specta_type};
use crate::type_transform::TypeTransform;

pub struct FieldSummary {
    pub requires_serde_as: bool,
}

pub fn transform_fields(
    fields: &mut Punctuated<Field, Token![,]>,
    serde_enabled: bool,
    specta_enabled: bool,
) -> FieldSummary {
    let mut summary = FieldSummary {
        requires_serde_as: false,
    };

    for field in fields {
        let transform = TypeTransform::from_type(&field.ty);
        if serde_enabled && transform.is_option {
            ensure_serde_default(&mut field.attrs);
        }
        if serde_enabled && let Some(adapter) = transform.serde_adapter {
            ensure_serde_adapter(&mut field.attrs, adapter);
            summary.requires_serde_as = true;
        }
        if specta_enabled && let Some(ty) = transform.specta_type {
            ensure_specta_type(&mut field.attrs, ty);
        }
    }

    summary
}
