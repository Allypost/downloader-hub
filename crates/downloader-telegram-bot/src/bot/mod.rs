pub mod helpers;

use std::string::ToString;

use app_actions::{
    actions::AVAILABLE_ACTIONS, downloaders::AVAILABLE_DOWNLOADERS,
    extractors::AVAILABLE_EXTRACTORS, fixers::AVAILABLE_FIXERS,
};
use app_config::Config;
use helpers::status_message::StatusMessage;
use once_cell::sync::OnceCell;
use teloxide::{
    adaptors::trace,
    prelude::*,
    requests::RequesterExt,
    types::{LinkPreviewOptions, ParseMode, ReplyParameters},
    utils::command::BotCommands,
};
use tracing::{field, info, trace, Instrument, Span};
use url::Url;

use crate::queue::{task::Task, TaskQueue};

pub type TeloxideBot =
    teloxide::adaptors::CacheMe<trace::Trace<teloxide::adaptors::DefaultParseMode<teloxide::Bot>>>;

static TELEGRAM_BOT: OnceCell<TeloxideBot> = OnceCell::new();

pub struct TelegramBot;
impl TelegramBot {
    pub fn instance() -> &'static TeloxideBot {
        TELEGRAM_BOT.get_or_init(|| {
            let tg_config = Config::global().telegram_bot();

            let api_url = Url::parse(&tg_config.api_url).expect("Invalid API URL");

            teloxide::Bot::new(&tg_config.bot_token)
                .set_api_url(api_url)
                .parse_mode(ParseMode::Html)
                .trace(trace::Settings::TRACE_EVERYTHING)
                .cache_me()
        })
    }

    pub fn pure_instance() -> &'static teloxide::Bot {
        Self::instance().inner().inner().inner()
    }
}

#[derive(BotCommands, Debug, Clone)]
#[command(
    rename_rule = "snake_case",
    description = "These commands are supported:"
)]
enum BotCommand {
    #[command(description = "Display this text.")]
    Help,
    #[command(description = "Start using the bot.")]
    Start,
    #[command(description = "Print some info about the bot.")]
    About,
    #[command(description = "List available extractors.")]
    ListExtractors,
    #[command(description = "List available downloaders.")]
    ListDownloaders,
    #[command(description = "List available fixers.")]
    ListFixers,
    #[command(description = "List available actions.")]
    ListActions,
    #[command(description = "Responds with 'Pong!'")]
    Ping,
    // #[command(
    //     description = "Split the video into scenes (best effort). Must be a reply to a video \
    //                    message or text of a video message."
    // )]
    // SplitScenes,
}

pub async fn run() -> anyhow::Result<()> {
    info!("Starting command bot...");

    let bot = TelegramBot::instance();
    let me = bot.get_me().await?;

    bot.set_my_commands(BotCommand::bot_commands())
        .send()
        .await
        .expect("Failed to set commands");

    info!(api_url = ?TelegramBot::pure_instance().api_url().as_str(), id = ?me.id, user = ?me.username(), name = ?me.full_name(), "Bot started");

    Box::pin(
        Dispatcher::builder(bot, Update::filter_message().endpoint(answer))
            .build()
            .dispatch(),
    )
    .await;

    Ok(())
}

#[tracing::instrument(name = "message", skip(_bot, msg), fields(chat = %msg.chat.id, msg_id = %msg.id, with = field::Empty))]
async fn answer(_bot: &TeloxideBot, msg: Message) -> ResponseResult<()> {
    trace!(?msg, "Got message");

    tokio::task::spawn(
        async move {
            {
                let name = msg
                    .chat
                    .username()
                    .map(|x| format!("@{}", x))
                    .or_else(|| msg.chat.title().map(ToString::to_string))
                    .or_else(|| {
                        let mut name = String::new();
                        if let Some(first_name) = msg.chat.first_name() {
                            name.push_str(first_name);
                        }
                        if let Some(last_name) = msg.chat.last_name() {
                            name.push(' ');
                            name.push_str(last_name);
                        }

                        Some(name)
                    });

                if let Some(name) = name {
                    Span::current().record("with", field::debug(name));
                }
            }

            let bot_me = TelegramBot::instance().get_me().await?;

            let msg_text = msg
                .text()
                .or_else(|| msg.caption())
                .map(ToString::to_string)
                .unwrap_or_default();

            match BotCommand::parse(&msg_text, bot_me.username()) {
                Ok(c) => handle_command(msg, c).await,
                Err(_) => handle_message(msg).await,
            }
        }
        .instrument(Span::current()),
    );

    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn handle_command(msg: Message, command: BotCommand) -> ResponseResult<()> {
    info!(?command, "Handling command");
    match command {
        BotCommand::Help => {
            TelegramBot::instance()
                .send_message(msg.chat.id, BotCommand::descriptions().to_string())
                .reply_parameters(ReplyParameters::new(msg.id).allow_sending_without_reply())
                .await?;
        }
        BotCommand::Start => {
            TelegramBot::instance()
                .send_message(
                    msg.chat.id,
                    "Hello! I'm a bot that can help download your memes.\n\nJust send me a link \
                     to a funny video and I'll do the rest!\nYou can also just send or forward a \
                     message with media and links to me and I'll fix it up for you!\n\nIf you'd \
                     like to know more use the /help or /about commands.",
                )
                .reply_parameters(ReplyParameters::new(msg.id).allow_sending_without_reply())
                .await?;
        }
        BotCommand::About => {
            let tg_config = Config::global().telegram_bot();

            let text = tg_config.about.clone().unwrap_or_else(|| {
                let mut paragraphs = vec![
                    r#"This bot is a part of the <a href="https://github.com/Allypost/downloader-hub/">Downloader Hub project</a>. It's a bot that helps you download your memes"#.to_string(),
                    "It is powered by Rust, yt-dlp, ffmpeg, and some external services.".to_string(),
                    "The source code is available at\nhttps://github.com/Allypost/downloader-hub/tree/main/crates/downloader-telegram-bot"
                        .to_string(),
                    "You can find out about the available downloaders and fixers, and what they do by using the /list_extractors, /list_downloaders and /list_fixers commands."
                    .to_string(),
                    "No data about downloading/users is stored outside of logs that live in RAM".to_string(),
                ];

                if let Some(owner_link) = tg_config.owner_link() {
                    paragraphs.push(format!(
                        r#"This bot instance is ran by <a href="{link}">this user</a>."#,
                        link = owner_link,
                    ));
                }

                paragraphs.join("\n\n")
            });

            trace!(?text, "Sending about message");

            TelegramBot::instance()
                .send_message(msg.chat.id, text.trim())
                .reply_parameters(ReplyParameters::new(msg.id).allow_sending_without_reply())
                .link_preview_options(LinkPreviewOptions {
                    is_disabled: true,
                    prefer_large_media: false,
                    prefer_small_media: false,
                    show_above_text: false,
                    url: None,
                })
                .await?;
        }
        BotCommand::ListExtractors => {
            let extractors_text = AVAILABLE_EXTRACTORS
                .iter()
                .map(|x| {
                    format!(
                        "<blockquote><u>{}</u>\n{}</blockquote>",
                        x.name(),
                        x.description()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            TelegramBot::instance()
                .send_message(
                    msg.chat.id,
                    format!(
                        "Extractors are used to get info about links. That info can then be \
                         passed to the downloaders who actually download the \
                         content.\n\nAvailable extractors:\n{}",
                        extractors_text
                    ),
                )
                .reply_parameters(ReplyParameters::new(msg.id).allow_sending_without_reply())
                .await?;
        }
        BotCommand::ListDownloaders => {
            let downloaders_text = AVAILABLE_DOWNLOADERS
                .iter()
                .map(|x| {
                    format!(
                        "<blockquote><u>{}</u>\n{}</blockquote>",
                        x.name(),
                        x.description()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            TelegramBot::instance()
                .send_message(
                    msg.chat.id,
                    format!(
                        "Downloaders are the things that actually download the content. They need \
                         to be given the exact info about what and how to download, which they \
                         get from extractors.\n\nAvailable downloaders:\n{}",
                        downloaders_text
                    ),
                )
                .reply_parameters(ReplyParameters::new(msg.id).allow_sending_without_reply())
                .await?;
        }
        BotCommand::ListFixers => {
            let fixers_text = AVAILABLE_FIXERS
                .iter()
                .map(|x| {
                    format!(
                        "<blockquote>{star}<u>{name}</u>\n{desc}</blockquote>",
                        name = x.name(),
                        desc = x.description(),
                        star = if x.enabled_by_default() { "" } else { "*" }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n\n* Not enabled by default";

            TelegramBot::instance()
                .send_message(
                    msg.chat.id,
                    format!(
                        "Fixers are used to fix up the content somewhere on disk in various \
                         ways.\n\nAvailable fixers:\n{}",
                        fixers_text
                    ),
                )
                .reply_parameters(ReplyParameters::new(msg.id).allow_sending_without_reply())
                .await?;
        }
        BotCommand::ListActions => {
            let actions_text = AVAILABLE_ACTIONS
                .iter()
                .map(|x| {
                    format!(
                        "<blockquote><u>{}</u>\n{}</blockquote>",
                        x.name(),
                        x.description()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            TelegramBot::instance()
                .send_message(
                    msg.chat.id,
                    format!(
                        "Actions are used to do something with the content.\n\nAvailable \
                         actions:\n{}",
                        actions_text
                    ),
                )
                .reply_parameters(ReplyParameters::new(msg.id).allow_sending_without_reply())
                .await?;
        }
        BotCommand::Ping => {
            TelegramBot::instance()
                .send_message(msg.chat.id, "Pong!")
                .reply_parameters(ReplyParameters::new(msg.id).allow_sending_without_reply())
                .await?;
        }
    }

    Ok(())
}

async fn handle_message(msg: Message) -> ResponseResult<()> {
    info!("Adding download request to queue");

    let mut status_message = StatusMessage::from_message(&msg);

    status_message
        .update_message("Message queued. Waiting for spot in line...")
        .await?;

    TaskQueue::push(Task::download_request(msg, status_message));

    Ok(())
}
