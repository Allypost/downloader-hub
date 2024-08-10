#[must_use]
pub fn value_parser_ensure_min_length(min_len: usize) -> impl clap::builder::TypedValueParser {
    move |s: &str| {
        if s.len() < min_len {
            return Err(format!(
                "Value must be at least 32 characters long. Currently it's {} characters long.",
                s.len()
            ));
        }

        Ok(s.to_string())
    }
}
