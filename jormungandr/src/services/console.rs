use async_trait::async_trait;
use organix::{
    service::Intercom, IntercomMsg, Service, ServiceIdentifier, ServiceState, WatchdogError,
};

/// console service, control standard output displays
/// (including the logs)
pub struct ConsoleService {
    state: ServiceState<Self>,
}

#[derive(Debug, IntercomMsg)]
pub struct ConsoleApi {
    message: Message,
}

#[derive(Debug)]
enum Message {
    Error {
        error: Box<dyn std::error::Error + Send>,
    },
}

impl ConsoleApi {
    pub async fn error<I>(
        intercom: &mut Intercom<ConsoleService>,
        error: I,
    ) -> Result<(), WatchdogError>
    where
        I: Into<Box<dyn std::error::Error + Send>>,
    {
        let message = Message::Error {
            error: error.into(),
        };

        intercom.send(Self { message }).await
    }
}

#[async_trait]
impl Service for ConsoleService {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "console";

    type IntercomMsg = ConsoleApi;

    fn prepare(state: ServiceState<Self>) -> Self {
        Self { state }
    }

    async fn start(mut self) {
        let recv = self.state.intercom_mut();

        while let Some(message) = recv.recv().await {
            match message {
                Message::Error { error } => {
                    eprintln!("{}", error);
                }
            }
        }
    }
}
