use teloxide::types::{Message, MessageEntityKind};
use url::Url;

#[tracing::instrument(skip_all, fields(msg = %msg.id))]
pub fn urls_in_message(msg: &Message) -> Vec<Url> {
    let entities = msg
        .parse_entities()
        .or_else(|| msg.parse_caption_entities())
        .unwrap_or_default();

    entities
        .iter()
        .filter_map(|x| match x.kind() {
            MessageEntityKind::Url => Url::parse(x.text()).ok(),
            MessageEntityKind::TextLink { url } => Some(url.clone()),
            _ => None,
        })
        .collect()
}
