use std::error::Error;

use clap::{Parser, Subcommand};
use actix::prelude::*;
use serde::{Serialize, Deserialize};


/// This structure are the command line parameters passed to the system from the command line.
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
pub struct Cli {
    /// System Mode
    #[clap(subcommand)]
    pub mode: Modes,

    /// ip address of the GPSD server
    #[clap(default_value = "127.0.0.1", long)]
    pub gpsd_server: String,

    /// gpsd port
    #[clap(default_value_t = 2947, long)]
    pub gpsd_port: u16,

    /// port to input and output data on (NMEA or RTCM, depending on mode)
    #[clap(default_value_t = 4223, long)]
    pub output_port: u16,

    /// GPS USB port
    #[clap(default_value = "/dev/ttyACM0", long)]
    pub gps_usb_port: String,

    /// Secondary GPS serial port
    #[clap(default_value = "/dev/ttyS0", long)]
    pub gps_tty_port: String,

    /// Default web port
    #[clap(default_value_t = 8080, long)]
    pub web_port: u16,

    /// Start data collection/corrections immediately.
    #[clap(default_value_t = false, long, action)]
    pub start: bool,
}

/// These are the settings associated with the various sub modes.
#[derive(Subcommand, Clone, Serialize, Deserialize)]
pub enum Modes {
    /// Set the system into RTK rover
    RTKRover {
        /// NTRIP server username
        #[clap(default_value="", long)]
        username: String,

        /// NTRIP server password
        #[clap(long, default_value="")]
        password: String,

        /// NTRIP server address
        #[clap(default_value = "rtk2go.com", long)]
        server: String,

        /// NTRIP mount point
        #[clap(long, default_value = "")]
        mount_point: String,

        /// NTRIP server port
        #[clap(default_value_t = 2101, long)]
        port: u16
    },
    /// Set the systen into  RTK base mode
    RTKBase {
        /// NTRIP server username
        #[clap(default_value="", long)]
        username: String,

        /// NTRIP server password
        #[clap(default_value = "", long)]
        password: String,

        /// NTRIP server address
        #[clap(default_value = "rtk2go.com", long)]
        server: String,

        /// NTRIP mount point
        #[clap(default_value = "", long)]
        mount_point: String,

        /// NTRIP server port
        #[clap(default_value_t = 2101, long)]
        port: u16
    },
    /// Set the system into post processing raw capture mode
    PPPMode {
        /// RINEX file output
        #[clap(default_value = "raw_data.obs", long)]
        filename: String,

        /// Interval between measurements in seconds
        #[clap(default_value_t = 30, long)]
        interval: u32,

        /// Number of captures
        #[clap(default_value_t = 2880, long)]
        number_of_collections: u32,
    },
    /// Run the system in standalone mode (simple GPS)
    Standalone,
}

/// Error types that can occur when setting the settings.
#[derive(Debug)]
pub enum SettingsError {
    InvalidMode,
}

impl Error for SettingsError{

}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SettingsError::InvalidMode => write! (f, "Mismatched Settings Modes."),
        }
    }
}


///This is the settings actor
#[derive(Message)]
#[rtype(result = "Result<Modes, SettingsError>")]
pub enum SettingsMessage {
    SetSettings(Modes),
    GetSettings()
}

/// This structure is an actor for handling the system settings. Individual settings
/// can be changes, but the overall mode cannot.
pub struct SettingsHandler {
    settings: Modes,
}

impl SettingsHandler {
    pub fn new(settings: Modes) -> Self {
        SettingsHandler{ settings: settings}
    }
}

impl Actor for SettingsHandler {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
       log::info!("Settings actor is alive");
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
       log::info!("Settings actor is stopped");
    }
}

impl Handler<SettingsMessage> for SettingsHandler {
    type Result = Result<Modes, SettingsError>;

    fn handle(&mut self, msg: SettingsMessage, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            SettingsMessage::SetSettings(settings) => {
                if std::mem::discriminant(&self.settings) != std::mem::discriminant (&settings) {
                    log::error!("Settings Modes did not match, the provided settings are for an incompatible mode.");
                    return Err(SettingsError::InvalidMode)
                }

                match settings {
                    Modes::PPPMode{filename, interval, number_of_collections} => {
                        //todo validate the inputs.
                        self.settings = Modes::PPPMode { filename: filename,
                                                         interval: interval,
                                                         number_of_collections: number_of_collections };
                    },
                    Modes::RTKBase{username, password, server, mount_point, port} => {
                        //todo validate the inputs
                        self.settings = Modes::RTKBase {
                            username: username,
                            password: password,
                            server: server,
                            mount_point: mount_point,
                            port: port
                        };
                    },
                    Modes::RTKRover{username, password, server, mount_point, port} => {
                        //todo validate the inputs
                        self.settings = Modes::RTKRover {
                            username: username,
                            password: password,
                            server: server,
                            mount_point: mount_point,
                            port: port
                        };
                    }
                    Modes::Standalone => ()
                }
                Ok(self.settings.clone())
            },
            SettingsMessage::GetSettings() => {
                Ok(self.settings.clone())
            }
        }
    }
}

