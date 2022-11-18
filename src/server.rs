use std::{
    future::Future,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use async_std::{
    channel::Sender,
    io::{prelude::BufReadExt, BufReader, WriteExt},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    stream::StreamExt,
    task,
};
use mailin::{Action, Session, SessionBuilder};

use crate::{error::Error, handler::TelegramMailHandler, telegram::TelegramBroker};

pub struct SMTPTelegramServerBuilder {
    host: SocketAddr,
    api_token: String,
    standard_chat_id: Option<u64>,
    telegram_bot_api_delay: Duration,
}

impl SMTPTelegramServerBuilder {
    pub fn new(api_token: impl Into<String>) -> SMTPTelegramServerBuilder {
        let api_token = api_token.into();

        Self {
            host: "0.0.0.0:17333".parse().unwrap(),
            api_token,
            standard_chat_id: None,
            telegram_bot_api_delay: Duration::from_secs(0),
        }
    }

    pub async fn with_host(
        mut self,
        host: impl ToSocketAddrs,
    ) -> Result<SMTPTelegramServerBuilder, Error> {
        self.host = host
            .to_socket_addrs()
            .await?
            .next()
            .ok_or(Error::SocketAddrParseError)?;
        Ok(self)
    }

    pub fn with_socket(mut self, host: impl Into<Option<SocketAddr>>) -> SMTPTelegramServerBuilder {
        if let Some(host) = host.into() {
            self.host = host;
        } else {
            self.host = "0.0.0.0:17333".parse().unwrap();
        }
        self
    }

    pub fn with_ip(mut self, ip: impl Into<IpAddr>) -> SMTPTelegramServerBuilder {
        let ip = ip.into();
        self.host.set_ip(ip);
        self
    }

    pub fn with_port(mut self, port: u16) -> SMTPTelegramServerBuilder {
        self.host.set_port(port);
        self
    }

    pub fn with_standard_chat_id(
        mut self,
        chat_id: impl Into<Option<u64>>,
    ) -> SMTPTelegramServerBuilder {
        self.standard_chat_id = chat_id.into();
        self
    }

    pub fn with_telegram_bot_api_delay(mut self, delay: Duration) -> SMTPTelegramServerBuilder {
        self.telegram_bot_api_delay = delay;
        self
    }

    pub async fn build(self) -> Result<SMTPTelegramServer, Error> {
        Ok(SMTPTelegramServer {
            listener: TcpListener::bind(self.host).await?,
            broker: TelegramBroker::new(
                self.api_token,
                self.telegram_bot_api_delay,
                self.standard_chat_id,
            ),
        })
    }
}

pub struct SMTPTelegramServer {
    listener: TcpListener,
    broker: TelegramBroker,
}

impl SMTPTelegramServer {
    async fn process_line(
        session: &mut Session<TelegramMailHandler>,
        line: &[u8],
    ) -> Result<(Option<Vec<u8>>, bool), Error> {
        log::debug!(
            "Process line: {:?}",
            String::from_utf8(line.to_vec()).unwrap()
        );
        let res = session.process(line);
        let mut response = Vec::new();

        match res.action {
            Action::Reply => {
                log::debug!("Action reply");
                res.write_to(&mut response)?;
                log::debug!("response: {:?}", String::from_utf8(response.clone()));
                return Ok((Some(response), false));
            }
            Action::Close => {
                log::debug!("Action close");
                res.write_to(&mut response)?;
                log::debug!("response: {:?}", String::from_utf8(response.clone()));
                if res.is_error {
                    log::error!("SMTP error");
                }
                return Ok((Some(response), true));
            }
            Action::NoReply => log::debug!("Action noreply"),
            Action::UpgradeTls => log::debug!("Action upgrade"), // No response needed
        }
        Ok((None, false))
    }

    async fn connection_loop(
        stream: TcpStream,
        sender: Sender<String>,
    ) -> Result<(), Error> {
        let mut session = SessionBuilder::new("Mail_to_telegram")
            .build(stream.peer_addr()?.ip(), TelegramMailHandler::new(sender));
        let stream = Arc::new(stream);
        {
            let mut stream = &*stream;
            let greeting = session.greeting();
            let mut response = Vec::new();
            greeting.write_to(&mut response)?;
            stream.write_all(response.as_slice()).await?;
        }
        log::debug!("Reader creation");
        let reader = BufReader::new(&*stream); // 2
        log::debug!("Reader created");
        let mut lines = reader.lines();

        log::debug!("Enter read loop");

        while let Some(line) = lines.next().await {
            // 4
            let line = format!("{}\r\n", line?);
            let (response, is_closing) = Self::process_line(&mut session, line.as_bytes()).await?;
            if let Some(res) = response {
                let mut stream = &*stream;
                stream.write_all(res.as_slice()).await?;
            }
            if is_closing {
                log::debug!("Connection closing");
                break;
            }
        }
        Ok(())
    }

    fn spawn_and_log_error<F>(&self, fut: F) -> task::JoinHandle<()>
    where
        F: Future<Output = Result<(), Error>> + Send + 'static,
    {
        task::spawn(async move {
            if let Err(e) = fut.await {
                log::error!("{}", e)
            }
        })
    }

    pub async fn listen(&self) -> Result<(), Error> {
        log::info!("Server started...");
        self.broker.serve().await;
        let mut incoming = self.listener.incoming();
        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            log::debug!("Accepting from: {}", stream.peer_addr()?);
            let _handle = task::spawn(
                self.spawn_and_log_error(Self::connection_loop(stream, self.broker.get_sender())),
            );
        }
        Ok(())
    }
}
