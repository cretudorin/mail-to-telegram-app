use std::{sync::Arc, time::Duration};

use lazy_static::lazy_static;
use regex::Regex;
use reqwest::{header, multipart::{Form, Part}, Client};
use serde::Serialize;
use async_std::{
    task,
    channel::{Sender, Receiver, bounded},
};

use crate::handler::Message;

lazy_static! {
    pub static ref CHAT_ID_REGEX: Regex = Regex::new(r"(\d*)@telegram-bot\.com").unwrap();
}

#[derive(Serialize)]
struct TelegramSendMessage<'a> {
    chat_id: u64,
    text: &'a str,
}

impl<'a> TelegramSendMessage<'a> {
    fn new(chat_id: u64, text: &'a str) -> Self {
        Self { chat_id, text }
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
        let (sen, recv) = bounded(5000);
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
        let standard_chat_id = self.standard_chat_id;
        let wait_duration = self.api_call_delay;

        while let Ok(msg) = self.recv.recv().await {
            log::debug!("Mail recieved");
            for recipient in msg.recipients {
                if let Some(chat_id) = Self::parse_chat_id(&recipient, standard_chat_id) {
                    log::debug!("Chatid present");
                    let body =
                    if msg.text.len() < 4096 {
                        let tmsg = TelegramSendMessage::new(chat_id, &msg.text);
                        serde_json::to_string(&tmsg)
                            .map_err(|e| {
                                log::error!(
                                "Constructing normal telegram message payload failed, error: {:?}",
                                e
                            );
                                e
                            })
                            .ok()
                    } else {
                        log::info!(
                            "Mail needs to be send as text file, because of the size of {}",
                            msg.text.len()
                        );
                        let tmsg = Form::new()
                            .text("caption", "Mail was too large, sent as text file")
                            .text("chat_id", chat_id.to_string())
                            .part("document", Part::bytes(msg.text.clone().into_bytes()).file_name("mail.txt").mime_str("text/plain").unwrap());
                        Some(tmsg.boundary().to_string())
                    };
                    if let Some(tmsg) = body {
                        log::info!("Send mail over telegram to chat id '{:?}' ", chat_id,);
                        let url = self.bot_url.clone();
                        task::spawn(async move {
                            let client = Client::new();
                            let res = client
                                .post(&*url)
                                .body(tmsg)
                                .header(header::CONTENT_TYPE, "application/json")
                                .send()
                                .await;
                            if let Ok(resp) = res {
                                log::info!(
                                    "Request arrived at telegram bot api, response api code: {}",
                                    resp.status()
                                );
                                log::debug!("API response: {:?}", resp.text().await);
                            } else if let Err(e) = res {
                                log::error!("Telegram bot api could not be called, error: {e}")
                            }
                        });
                    }
                } else {
                    log::warn!("Mail had to be disregarded because chat_id could not been optained by the recipient email and no fall back chat id was supplied. Email was {}", recipient);
                }
            }

            task::sleep(wait_duration).await;
        }
    }
}
