#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recipient {
    Phone(String),
    BusinessScopedUserId(String),
    Individual {
        phone: Option<String>,
        business_scoped_user_id: Option<String>,
    },
    Group(String),
}

impl Recipient {
    pub fn phone(phone: impl Into<String>) -> Self {
        Self::Phone(phone.into())
    }

    pub fn bsuid(id: impl Into<String>) -> Self {
        Self::BusinessScopedUserId(id.into())
    }

    pub fn individual(
        phone: Option<impl Into<String>>,
        business_scoped_user_id: Option<impl Into<String>>,
    ) -> Self {
        Self::Individual {
            phone: phone.map(Into::into),
            business_scoped_user_id: business_scoped_user_id.map(Into::into),
        }
    }

    pub fn group(group_id: impl Into<String>) -> Self {
        Self::Group(group_id.into())
    }

    pub fn is_group(&self) -> bool {
        matches!(self, Self::Group(_))
    }

    pub fn send_to(&self) -> Option<&str> {
        match self {
            Self::Phone(phone) => Some(phone),
            Self::BusinessScopedUserId(_) => None,
            Self::Individual { phone, .. } => phone.as_deref(),
            Self::Group(group_id) => Some(group_id),
        }
    }

    pub fn business_scoped_user_id(&self) -> Option<&str> {
        match self {
            Self::Phone(_) | Self::Group(_) => None,
            Self::BusinessScopedUserId(id) => Some(id),
            Self::Individual {
                business_scoped_user_id,
                ..
            } => business_scoped_user_id.as_deref(),
        }
    }
}
