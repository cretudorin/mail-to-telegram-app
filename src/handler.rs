use std::{io, net::IpAddr};

use async_std::channel::Sender;
use mailin::{response::{OK, INTERNAL_ERROR}, Handler, Response};

#[derive(Debug)]
pub struct TelegramMailHandler {
    msg: Vec<u8>,
    sender: Sender<String>,
}

impl TelegramMailHandler {
    pub fn new(sender: Sender<String>) -> Self {
        Self {
            msg: Vec::new(),
            sender,
        }
    }
}

impl Handler for TelegramMailHandler {
    fn helo(&mut self, ip: IpAddr, domain: &str) -> mailin::Response {
        log::info!("HELO: {:?} {}", ip, domain);
        OK
    }

    /// Called when a data command is received
    fn data_start(&mut self, domain: &str, from: &str, is8bit: bool, to: &[String]) -> Response {
        log::info!("DATA start: {} {} {:?} {}", domain, from, to, is8bit);
        self.msg = Vec::new();
        OK
    }

    /// Called when a data buffer is received
    fn data(&mut self, buf: &[u8]) -> io::Result<()> {
        self.msg.append(&mut buf.to_vec());
        log::info!("DATA: {:?}", String::from_utf8(buf.to_vec()).ok());
        Ok(())
    }

    /// Called at the end of receiving data
    fn data_end(&mut self) -> Response {
        log::info!("Data end");
        let msg = String::from_utf8(self.msg.clone());
        if let Ok(msg) = msg {
            log::info!("FULL EMAIL: {}", msg);
            if let Err(e) = self.sender.send_blocking(msg) {
                log::error!("Telegram msg broker error: {:?}", e);
                return INTERNAL_ERROR;
            }
        }
        OK
    }
}
