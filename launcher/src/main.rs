#[cfg(feature = "client")]
mod client;
#[cfg(all(feature = "client", feature = "server"))]
mod host_server;
#[cfg(all(feature = "client", feature = "server"))]
mod separate;
#[cfg(feature = "server")]
mod server;
/// Provides a CLI to start the app in different modes
pub(crate) mod settings;

use bevy::prelude::*;
use clap::{Parser, Subcommand};

/// CLI options to create an [`App`]
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub mode: Option<Mode>,
}

#[derive(Subcommand, Debug)]
pub enum Mode {
    #[cfg(feature = "client")]
    /// Runs the app in client mode
    Client {
        #[arg(short, long, default_value = None)]
        client_id: Option<u64>,
    },
    #[cfg(feature = "server")]
    /// Runs the app in server mode
    Server,
    #[cfg(all(feature = "client", feature = "server"))]
    /// Creates two bevy apps: a client app and a server app.
    /// Data gets passed between the two via channels.
    Separate {
        #[arg(short, long, default_value = None)]
        client_id: Option<u64>,
    },
    #[cfg(all(feature = "client", feature = "server"))]
    /// Run the app in host-server mode.
    /// The client and the server will run inside the same app. The peer acts both as a client and a server.
    HostServer {
        #[arg(short, long, default_value = None)]
        client_id: Option<u64>,
    },
}

fn run(cli: Cli) {
    match cli.mode {
        #[cfg(all(feature = "client", feature = "server"))]
        Some(Mode::HostServer { client_id }) => {
            let mut app = host_server::HostServer::new(client_id.unwrap_or(0));
            app.run();
        }
        #[cfg(all(feature = "client", feature = "server"))]
        Some(Mode::Separate { client_id }) => {
            let mut app = separate::Separate::new(client_id.unwrap_or(0));
            app.run();
        }
        #[cfg(feature = "client")]
        Some(Mode::Client { client_id }) => {
            let mut app = client::ClientApp::new(client_id.unwrap_or(0));
            app.run();
        }
        #[cfg(feature = "server")]
        Some(Mode::Server) => {
            let mut app = server::ServerApp::new();
            app.run();
        }
        None => {
            #[cfg(all(feature = "client", feature = "server"))]
            run(Cli {
                mode: Some(Mode::HostServer { client_id: None }),
            });
            #[cfg(all(feature = "server", not(feature = "client")))]
            run(Cli {
                mode: Some(Mode::Server),
            });

            #[cfg(all(feature = "client", not(feature = "server")))]
            run(Cli {
                mode: Some(Mode::Client { client_id: None }),
            });
        }
    }
}

fn main() {
    let cli = Cli::parse();
    run(cli);
}
