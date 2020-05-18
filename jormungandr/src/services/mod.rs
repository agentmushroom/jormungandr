//! This module defines all the different services available
//! in jormungandr.
//!
mod config;
mod console;
mod monitor;

use organix::{service::ServiceManager, Organix, WatchdogBuilder};

pub use self::{
    config::{ConfigApi, ConfigService},
    console::ConsoleService,
    monitor::MonitorService,
};

/// All services of the Jörmungandr app to be added in this field.
///
/// By default all services are going to use a _shared_ runtime
/// with `io` and `time` driver (from tokio) already enabled.
///
/// However, consider using `#[runtime(io, time)]` for the service
/// who need their own runtime defined.
#[derive(Organix)]
#[runtime(shared)]
struct JormungandrApp {
    /// Node's monitoring service
    ///
    /// This is responsible to boot the other services and
    /// to keep them up and running as it is necessary
    monitoring: ServiceManager<MonitorService>,
    /// Node's configuration service
    ///
    /// the configuration service can run on the shared runtime as
    /// it is supposed to be lightweight enough.
    configuration: ServiceManager<ConfigService>,
}

/// services entry point
///
/// This function will block until the end of the application runtime
pub fn entry() {
    // build the watchdog monitor
    let watchdog = WatchdogBuilder::<JormungandrApp>::new().build();

    // the controller to spawn the initial services
    let mut controller = watchdog.control();

    watchdog.spawn(async move {
        controller
            .start::<MonitorService>()
            .await
            .expect("Cannot start the Monitoring service");
    });

    watchdog.wait_finished()
}
