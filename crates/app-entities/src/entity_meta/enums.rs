use crate::sea_orm_active_enums::ItemStatus;

impl ItemStatus {
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }
}
