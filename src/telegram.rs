use std::{sync::Arc, time::Duration};

use async_std::{
    channel::{unbounded, Receiver, Sender},
    task,
};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;
use surf::http::mime;

use crate::handler::Message;

lazy_static! {
    pub static ref CHAT_ID_REGEX: Regex = Regex::new(r"(\d*)@telegram-bot\.com").unwrap();
}

#[derive(Serialize)]
struct TelegramSendMessage<'a> {
    chat_id: u64,
    text: &'a str,
    parse_mode: &'static str,
}

impl<'a> TelegramSendMessage<'a> {
    fn new(chat_id: u64, text: &'a str) -> Self {
        Self {
            chat_id,
            text,
            parse_mode: "HTML",
        }
    }
}

pub struct TelegramBroker {
    sen: Sender<Message>,
    recv: Receiver<Message>,
    bot_url: Arc<String>,
    api_call_delay: Duration,
    standard_chat_id: Option<u64>,
}

impl TelegramBroker {
    pub fn new(api_token: String, api_call_delay: Duration, standard_chat_id: Option<u64>) -> Self {
        let (sen, recv) = unbounded();
        Self {
            sen,
            recv,
            bot_url: Arc::new(format!(
                "https://api.telegram.org/bot{}/sendMessage",
                api_token
            )),
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
        let r = self.recv.clone();
        let url = self.bot_url.clone();
        let standard_chat_id = self.standard_chat_id;
        let wait_duration = self.api_call_delay;
        task::spawn(async move {
            while let Ok(msg) = r.recv().await {
                log::debug!("Mail recieved");
                for recipient in msg.recipients {
                    if let Some(chat_id) = Self::parse_chat_id(&recipient, standard_chat_id) {
                        log::debug!("Chatid present");
                        let tmsg = TelegramSendMessage::new(chat_id, &msg.text);
                        let tmsg = serde_json::to_string(&tmsg);
                        if let Ok(tmsg) = tmsg {
                            log::info!("Send mail over telegram to chat id: {:?}", chat_id);
                            surf::post(&*url)
                                .body(tmsg)
                                .content_type(mime::JSON)
                                .await
                                .ok();
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
