use std::{time::Duration, sync::Arc};

use async_std::{
    channel::{unbounded, Receiver, Sender},
    task,
};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;
use surf::http::mime;

lazy_static! {
    pub static ref MESSAGE_REGEX: Regex = Regex::new(
        r"To: (.+)\nSubject: (.+)\r\nDate: (.+)\r\nFrom: (.+)\r\nMessage-Id: (.+)\r\n([\s\S]*)"
    )
    .unwrap();
    pub static ref CHAT_ID_REGEX: Regex = Regex::new(r"(\d*)@telegram-bot\.com").unwrap();
}

#[derive(Serialize)]
struct TelegramSendMessage {
    chat_id: u64,
    text: String,
    parse_mode: String,
}

impl TelegramSendMessage {
    fn new(chat_id: u64, text: String) -> Self {
        Self {
            chat_id,
            text,
            parse_mode: "HTML".into(),
        }
    }
}

pub struct TelegramBroker {
    sen: Sender<String>,
    recv: Receiver<String>,
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
            bot_url: Arc::new(format!("https://api.telegram.org/bot{}/sendMessage", api_token)),
            api_call_delay,
            standard_chat_id,
        }
    }

    pub fn get_sender(&self) -> Sender<String> {
        self.sen.clone()
    }

    fn parse_chat_id(email: &str, standard_chat_id: Option<u64>) -> Option<u64> {
        let chat_id = CHAT_ID_REGEX.captures(email).and_then(|cap| cap[1].parse::<u64>().ok());
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
                let cap = MESSAGE_REGEX.captures(&msg);
                if let Some(cap) = cap {
                    let to = &cap[1];
                    let subject = &cap[2];
                    let date = &cap[3];
                    let from = &cap[4];
                    let message = &cap[6].replace('\r', "\n");
                    let msg = format!("Subject: <b>{subject}</b>\nDate: <b>{date}</b>\nFrom: <b>{from}</b>\n{message}");
                    log::debug!("Mail parsed");
                    if let Some(chat_id) = Self::parse_chat_id(to, standard_chat_id) {
                        log::debug!("Chatid present");
                        let tmsg =
                            TelegramSendMessage::new(chat_id, msg);
                        let tmsg = serde_json::to_string(&tmsg);
                        if let Ok(tmsg) = tmsg {
                            log::debug!("Send Telegram message: {tmsg}");
                            surf::post(&*url)
                                .body(tmsg)
                                .content_type(mime::JSON)
                                .await
                                .ok();
                        }
                    }
                }
                task::sleep(wait_duration).await;
            }
        });
    }
}
