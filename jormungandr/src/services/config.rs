use crate::settings::{
    start::{Error, RawSettings, Settings},
    CommandLine,
};
use async_trait::async_trait;
use organix::{
    service::Intercom, IntercomMsg, Service, ServiceIdentifier, ServiceState, WatchdogError,
};
use std::sync::Arc;
use tokio::sync::oneshot;

/// Communicate with the configuration service
///
/// see the different _commands_ available
#[derive(IntercomMsg)]
pub struct ConfigApi {
    reply: oneshot::Sender<Arc<Settings>>,
}

/// the Configuration service
///
/// This service is responsible to load the configuration of the node
/// and to keep inform other nodes if the configuration has changed.
///
/// ## TODO
///
/// - [ ] allow dynamic modification of the settings,
/// - [ ] add interface to allow other service to register on settings
///       modifications.
///
pub struct ConfigService {
    state: ServiceState<Self>,
}

impl ConfigApi {
    /// attempt to query the Configuration Service for the settings
    ///
    pub async fn query_settings(
        intercom: &mut Intercom<ConfigService>,
    ) -> Result<Arc<Settings>, WatchdogError> {
        let (reply, receiver) = oneshot::channel();

        let query = ConfigApi { reply };

        intercom.send(query).await?;

        match receiver.await {
            Ok(obj) => Ok(obj),
            Err(err) => unreachable!(
                "It appears the ConfigService is up but not responding: {}",
                err
            ),
        }
    }
}

#[async_trait]
impl Service for ConfigService {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "configuration";

    type IntercomMsg = ConfigApi;

    fn prepare(state: ServiceState<Self>) -> Self {
        Self { state }
    }

    async fn start(self) {
        // load the command line options
        let command_line = CommandLine::load();

        // gentle hack
        if command_line.full_version {
            println!("{}", env!("FULL_VERSION"));
            std::process::exit(0);
        } else if command_line.source_version {
            println!("{}", env!("SOURCE_VERSION"));
            std::process::exit(0);
        }

        if let Err(err) = start(self.state, command_line).await {
            unimplemented!("{:#?}", err)
        }
    }
}

async fn start(state: ServiceState<ConfigService>, command_line: CommandLine) -> Result<(), Error> {
    let mut state = state;
    let raw_settings = RawSettings::load(command_line)?;
    let settings = raw_settings.try_into()?;

    let settings = Arc::new(settings);

    while let Some(ConfigApi { reply }) = state.intercom_mut().recv().await {
        let settings = Arc::clone(&settings);
        if let Err(_) = reply.send(settings) {
            // this case should not happen as we control the flow
            // for awaiting for the reply. So if the settings
            // is required the other end will wait until it
            // receives this reply
            //
            // Anyhow, we can still ignore the error
        }
    }

    Ok(())
}

impl std::fmt::Debug for ConfigApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigApi").field("reply", &"..").finish()
    }
}
