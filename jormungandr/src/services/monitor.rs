use crate::services::ConfigService;
use async_trait::async_trait;
use organix::{
    service::{NoIntercom, Status},
    Service, ServiceIdentifier, ServiceState, WatchdogError, WatchdogQuery,
};

/// the Monitoring service
///
/// This service is responsible to start and keep all the different service
/// up as needed.
///
pub struct MonitorService {
    state: ServiceState<Self>,
}

impl MonitorService {
    async fn boot(&mut self) -> Result<(), WatchdogError> {
        let mut watchdog = self.state.watchdog_controller().clone();

        self.start::<ConfigService>(&mut watchdog).await?;

        Ok(())
    }

    async fn start<T: Service>(
        &mut self,
        watchdog: &mut WatchdogQuery,
    ) -> Result<(), WatchdogError> {
        let mut number_attempts = 0;
        const MAX_NUMBER_ATTEMPTS: usize = 2;

        loop {
            let status = watchdog.status::<T>().await?;

            if number_attempts >= MAX_NUMBER_ATTEMPTS {
                return Err(WatchdogError::CannotStartService {
                    service_identifier: T::SERVICE_IDENTIFIER,
                    source: organix::service::ServiceError::CannotStart {
                        status: status.status,
                    },
                });
            } else {
                number_attempts += 1;
            }
            match status.status {
                Status::Shutdown { .. } => {
                    watchdog.start::<T>().await?;
                }
                Status::ShuttingDown { .. } => {
                    // wait
                    tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
                }
                Status::Starting { .. } => {
                    // wait
                    tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
                }
                Status::Started { .. } => {
                    break;
                }
            }
        }

        Ok(())
    }

    async fn shutdown_with_error(&mut self, error: impl std::error::Error) {
        let mut watchdog = self.state.watchdog_controller().clone();

        // tracing::error!(error, "shuting down with error");

        watchdog.shutdown().await
    }
}

#[async_trait]
impl Service for MonitorService {
    const SERVICE_IDENTIFIER: ServiceIdentifier = "monitoring";

    type IntercomMsg = NoIntercom;

    fn prepare(state: ServiceState<Self>) -> Self {
        Self { state }
    }

    async fn start(mut self) {
        if let Err(error) = self.boot().await {
            self.shutdown_with_error(error).await
        }

        // todo, monitor statuses?
    }
}
