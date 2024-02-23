pub fn option_contains<T: Eq>(option: &Option<T>, contains: &T) -> bool {
    option.as_ref().map_or(false, |value| contains == value)
}
