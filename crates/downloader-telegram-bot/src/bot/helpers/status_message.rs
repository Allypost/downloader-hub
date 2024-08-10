use teloxide::{
    payloads::{EditMessageTextSetters, SendMessageSetters},
    requests::Requester,
    types::{ChatId, Message, MessageId},
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

    pub async fn send_new_message(
        &mut self,
        text: &str,
        use_as_new_status_message: bool,
    ) -> Result<(), teloxide::RequestError> {
        let status_msg = TelegramBot::instance()
            .send_message(self.chat_id, text)
            .reply_to_message_id(self.msg_id)
            .allow_sending_without_reply(true)
            .await;

        let status_msg = match status_msg {
            Ok(msg) => msg,
            Err(e) => {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                return Err(e);
            }
        };

        if use_as_new_status_message {
            self.reply_msg_id = Some(status_msg.id);
        }

        Ok(())
    }

    pub async fn update_message(&mut self, text: &str) -> Result<(), teloxide::RequestError> {
        for _ in 0..3 {
            match self.reply_msg_id {
                Some(reply_id) => {
                    let res = TelegramBot::instance()
                        .edit_message_text(self.chat_id, reply_id, text)
                        .disable_web_page_preview(true)
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
                    let status_msg = TelegramBot::instance()
                        .send_message(self.chat_id, text)
                        .reply_to_message_id(self.msg_id)
                        .allow_sending_without_reply(true)
                        .await;

                    let status_msg = match status_msg {
                        Ok(msg) => msg,
                        Err(e) => {
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            return Err(e);
                        }
                    };

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
