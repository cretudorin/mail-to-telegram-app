use std::{io, mem, net::IpAddr};

use async_std::channel::Sender;
use mailin::{
    response::{INTERNAL_ERROR, OK},
    Handler, Response,
};

#[derive(Debug, Default)]
pub struct Message {
    pub sender: String,
    pub recipients: Vec<String>,
    pub text: String,
}

impl Message {
    pub fn new(sender: String, recipients: Vec<String>, text: String) -> Self {
        Self {
            sender,
            recipients,
            text,
        }
    }
}

#[derive(Debug)]
pub struct TelegramMailHandler {
    from: String,
    recipients: Vec<String>,
    data: Vec<u8>,
    sender: Sender<Message>,
}

impl TelegramMailHandler {
    pub fn new(sender: Sender<Message>) -> Self {
        Self {
            data: Vec::new(),
            recipients: Vec::new(),
            from: String::default(),
            sender,
        }
    }
}

impl Handler for TelegramMailHandler {
    fn helo(&mut self, _ip: IpAddr, _domainn: &str) -> mailin::Response {
        OK
    }

    /// Called when a data command is received
    fn data_start(&mut self, _domain: &str, from: &str, _is8bit: bool, to: &[String]) -> Response {
        log::debug!("Email start recieved, collect entire email...");
        self.data = Vec::new();
        self.from = from.to_string();
        self.recipients = to.to_vec();
        OK
    }

    /// Called when a data buffer is received
    fn data(&mut self, buf: &[u8]) -> io::Result<()> {
        self.data.append(&mut buf.to_vec());
        Ok(())
    }

    /// Called at the end of receiving data
    fn data_end(&mut self) -> Response {
        log::info!("Email recieved, encode and send to TelegramBroker");
        let msg = String::from_utf8(self.data.clone());
        if let Ok(msg) = msg {
            log::debug!("FULL EMAIL: {}", msg);
            // No clone take of from and recipients
            let recipients = mem::take(&mut self.recipients);
            let from = mem::take(&mut self.from);
            if let Err(e) = self
                .sender
                .send_blocking(Message::new(from, recipients, msg))
            {
                log::error!("Telegram msg broker error: {:?}", e);
                return INTERNAL_ERROR;
            }
        }
        OK
    }
}
