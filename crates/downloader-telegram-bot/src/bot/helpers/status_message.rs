use teloxide::{
    payloads::{EditMessageTextSetters, SendMessageSetters},
    requests::Requester,
    types::{ChatId, LinkPreviewOptions, Message, MessageId, ReplyParameters},
};

use crate::bot::TelegramBot;

#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct StatusMessage {
    chat_id: ChatId,
    msg_id: MessageId,
    reply_msg_id: Option<MessageId>,
}
impl StatusMessage {
    const fn new(chat_id: ChatId, msg_id: MessageId) -> Self {
        Self {
            chat_id,
            msg_id,
            reply_msg_id: None,
        }
    }

    pub const fn chat_id(&self) -> ChatId {
        self.chat_id
    }

    pub const fn msg_replying_to_id(&self) -> MessageId {
        self.msg_id
    }

    pub const fn from_message(msg: &Message) -> Self {
        Self::new(msg.chat.id, msg.id)
    }

    pub async fn send_additional_message(
        &self,
        text: &str,
    ) -> Result<Message, teloxide::RequestError> {
        TelegramBot::instance()
            .send_message(self.chat_id, text)
            .disable_notification(true)
            .reply_parameters(ReplyParameters::new(self.msg_id).allow_sending_without_reply())
            .await
    }

    pub async fn update_message(&mut self, text: &str) -> Result<(), teloxide::RequestError> {
        for _ in 0..3 {
            match self.reply_msg_id {
                Some(reply_id) => {
                    let res = TelegramBot::instance()
                        .edit_message_text(self.chat_id, reply_id, text)
                        .link_preview_options(LinkPreviewOptions {
                            is_disabled: true,
                            prefer_large_media: false,
                            prefer_small_media: false,
                            show_above_text: false,
                            url: None,
                        })
                        .await;

                    if matches!(
                        res,
                        Err(teloxide::RequestError::Api(
                            teloxide::ApiError::MessageToEditNotFound
                        ))
                    ) {
                        self.reply_msg_id = None;
                        continue;
                    }

                    return Ok(());
                }
                None => {
                    let status_msg = self.send_additional_message(text).await?;

                    self.reply_msg_id = Some(status_msg.id);

                    return Ok(());
                }
            }
        }

        Err(teloxide::RequestError::Api(
            teloxide::ApiError::MessageNotModified,
        ))
    }

    pub async fn delete_message(&self) -> Result<(), teloxide::RequestError> {
        if let Some(id) = self.reply_msg_id {
            TelegramBot::instance()
                .delete_message(self.chat_id, id)
                .await?;
        }

        Ok(())
    }
}

impl From<Message> for StatusMessage {
    fn from(msg: Message) -> Self {
        Self::from_message(&msg)
    }
}

impl<'a> From<&'a Message> for StatusMessage {
    fn from(msg: &'a Message) -> Self {
        Self::from_message(msg)
    }
}
