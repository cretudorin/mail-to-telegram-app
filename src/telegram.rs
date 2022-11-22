use std::{sync::Arc, time::Duration};

use async_std::{
    channel::{bounded, Receiver, Sender},
    task,
};
use lazy_static::lazy_static;
use regex::Regex;
use telegram_bot_api::{
    bot::BotApi,
    methods::{SendDocument, SendMessage},
    types::{ChatId, InputFile},
};

use crate::handler::Message;

lazy_static! {
    pub static ref CHAT_ID_REGEX: Regex = Regex::new(r"(\d*)@telegram-bot\.com").unwrap();
}

pub struct TelegramBroker {
    sen: Sender<Message>,
    recv: Receiver<Message>,
    bot: Arc<BotApi>,
    api_call_delay: Duration,
    standard_chat_id: Option<u64>,
}

impl TelegramBroker {
    pub async fn new(
        api_token: String,
        api_call_delay: Duration,
        standard_chat_id: Option<u64>,
    ) -> Self {
        let (sen, recv) = bounded(5000);
        Self {
            sen,
            recv,
            bot: Arc::new(
                BotApi::new(api_token, None)
                    .await
                    .expect("TELEGRAM_BOT_TOKEN incorrect"),
            ),
            api_call_delay,
            standard_chat_id,
        }
    }

    pub fn get_sender(&self) -> Sender<Message> {
        self.sen.clone()
    }

    fn parse_chat_id(email: &str, standard_chat_id: Option<u64>) -> Option<u64> {
        let chat_id = CHAT_ID_REGEX
            .captures(email)
            .and_then(|cap| cap[1].parse::<u64>().ok());
        chat_id.or(standard_chat_id)
    }

    pub async fn serve(&self) {
        let standard_chat_id = self.standard_chat_id;
        let wait_duration = self.api_call_delay;
        let recv = self.recv.clone();
        let bot = self.bot.clone();
        task::spawn(async move {
            while let Ok(msg) = recv.recv().await {
                log::debug!("Mail recieved");
                for recipient in msg.recipients {
                    if let Some(chat_id) = Self::parse_chat_id(&recipient, standard_chat_id) {
                        log::debug!("Chatid present");
                        let result = if msg.text.len() < 4096 {
                            let message =
                                SendMessage::new(ChatId::IntType(chat_id as i64), msg.text.clone());
                            bot.send_message(message).await
                        } else {
                            let mut message = SendDocument::new(
                                ChatId::IntType(chat_id as i64),
                                InputFile::FileBytes(
                                    "mail.txt".to_string(),
                                    msg.text.clone().into_bytes(),
                                ),
                            );
                            message.caption = Some("The mail was too long, send as text file".to_string());
                            bot.send_document(message).await
                        };
                        match result {
                            Ok(_) => log::info!("Mail successfully send to telegram bot"),
                            Err(e) => log::error!(
                                "Mail send over telegram failed due to this error: {:?}",
                                e
                            ),
                        }
                    } else {
                        log::warn!("Mail had to be disregarded because chat_id could not been optained by the recipient email and no fall back chat id was supplied. Email was {}", recipient);
                    }
                }

                task::sleep(wait_duration).await;
            }
        });
    }
}
