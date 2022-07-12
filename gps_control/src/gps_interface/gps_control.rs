use actix::prelude::*;
use actix::Message;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use port_scanner;
use std::process::Command;

const GPS_BAUDRATE: &str = "115200";
const UBLOX_VERSION: &str = "27.30";
const UBX_ACK: &str = "UBX-ACK-ACK:";
pub const GPS_DATA_DIR: &str= "data/";

/// GPSControl message, set GPS to either base station mode or rover mode. This also updates the system state and also sends
/// a state set message to the other device connected through LORA (putting it into the other state).
/// 
/// Base station mode switches the gps to stationary and sets the com port to outout RTCM correction messages
/// which get passed onto the LORA radio for sending to the rover. In this mode, the RTCM messages are retransmitted over 
/// a network socket.
/// 
/// Rover mode sends recieves RTCM correction messages from the LORA radio into the GPS through the serial port and
/// also retransmits any data recieved on the serial port to a socket for reading by other systems.
#[derive(Message, Debug)]
#[rtype(result="Result<(), Box<dyn std::error::Error + Send + Sync>>")]
pub enum GPSMode {
    Base(String /*username*/, String/*password*/, String/*server*/, String/*mount_point*/, u16/*port*/),
    Standalone,
    RAW (String /*filename*/, u32 /*interval*/, u32 /*number_of_collections*/),
    RtcmIn(String /*username*/, String/*password*/, String/*server*/, String/*mount_point*/, u16/*port*/),
    Stopped,
}


///GPS control strucutre, used to set up the the gpsd server through a combination of command line gpsctl and ubxtool commands.
pub struct GPSControl {
    ip_address: IpAddr,
    port: u16,
    gpsd_command: Option<std::process::Child>,
    ntrip_command: Option<std::process::Child>,
    nmea_publish_command: Option<std::process::Child>,
    rinex_collection_command: Option<std::process::Child>,
    gps_usb_port: String,
    io_port: u16,
}

impl GPSControl {
    /// Setup a new GPS control interface.
    ///  - ip_address: address of the GPSD daemon
    ///  - port: port of the GPSD daemon
    ///  - gps_usb_port: device/port name the gps usb is connected to
    ///  - io_port: local tcp port that NMEA is output on and RTCM input on in rover mode
    ///             or RTCM is output on in base station mode.
    pub fn new (ip_address: Option<&str>, 
                port: Option<u16>,
                gps_usb_port: Option<String>,
                io_port: Option<u16>) -> Self {

        let ip_address = match ip_address {
            Some(ip) => ip,
            None => "127.0.0.1"
        };
        let port = match port {
            Some(p) => p,
            None => 2947
        };
        let gps_usb_port = match gps_usb_port {
            Some(port) => port,
            None => "/dev/ttyACM0".to_string()
        };
        let io_port = match io_port {
            Some(port) => port,
            None => 4223
        };

        let ip_address = match ip_address.parse::<IpAddr>() {
            Ok(val) => val,
            Err(e) => {
                log::error!("Failed to parse input ip address: {:?}", e);
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
            }
        };
        GPSControl {
            ip_address: ip_address,
            port: port,
            gpsd_command: None,
            ntrip_command: None,
            nmea_publish_command: None,
            rinex_collection_command: None,
            gps_usb_port: gps_usb_port,
            io_port: io_port
        }    
    }

    ///Start collecting data in raw mode, saving the ubx data to a file.
    ///  - filename: filename to save the rinex observations to.
    ///  - interval_in_s: number of seconds to wait between collections.
    ///  - duration_in_min: number of minutes to collect data for.
    fn set_raw_mode(&self, filename: &str, interval_in_s: u32, number_of_collections: u32) {

        log::info!("Setting GPS into raw binary mode.");
        let _output = Command::new ("gpsctl").arg("-s").arg(GPS_BAUDRATE).output().expect("Failed to execute gpsctl.");
        log::info!("Baudrate set to {}.", GPS_BAUDRATE)
;            
        let ubx_commands = vec![
                            ("-p", "RESET"),
                            ("-d", "NMEA"),
                            ("-e", "BINARY"),
                            ("-e", "GLONASS"),
                            ("-d", "BEIDOU"),
                            ("-d", "GALILEO"),
                            ("-d", "SBAS"),
                            ("-e", "GPS"),
                            ("-e", "RAWX"),
                            ("-z", "CFG-MSGOUT-UBX_RXM_RAWX_USB,1")
                        ];
        GPSControl::run_ubx_commands(ubx_commands);
        
        //Kill the current runner before restarting, if running.
        if let Some(mut cmd) = self.rinex_collection_command.take() {
            cmd.kill().expect("gpsrinex couldn't be killed!");
        };

        
        self.rinex_collection_command = Some(Command::new ("gpsrinex")
                                .arg("-i").arg(interval_in_s.to_string())
                                .arg("-n").arg(number_of_collections.to_string())
                                .arg("-f").arg(filename)
                                .spawn().expect("Failed to execute gps2rinex."));
    
    }

    /// Set the rover into base station mode, enabling appropriate RTCM outputs and position modes.
    /// This is then followed by setting up the str2str process to stream those rtcm corrections.
    fn set_base_station_mode(&self, username: &str, password: &str, server: &str, mount_point: &str, port: u16) {
        log::info!("Setting the GPS into base station mode and setting the serial port TX to output RTCM messages.");
        let _output = Command::new ("gpsctl").arg("-s").arg(GPS_BAUDRATE).output().expect("Failed to execute gpsctl.");
        log::info!("Baudrate set to {}.", GPS_BAUDRATE);

        let ubx_commands = vec![
                            ("-p", "RESET"),
                            ("-e", "NMEA"),
                            ("-e", "BINARY"),
                            ("-e", "GLONASS"),
                            ("-d", "BEIDOU"),
                            ("-d", "GALILEO"),
                            ("-d", "SBAS"),
                            ("-e", "GPS"),
                            ("-d", "RAWX"),
                            ("-z", "CFG-NMEA-HIGHPREC,1"),
                            ("-z", "CFG-NAVSPG-DYNMODEL,2"), //Stationary mode
                            //("-z", "CFG-MSGOUT-UBX_RXM_RAWX_USB,1")
                        ];
        GPSControl::run_ubx_commands(ubx_commands);
    }

    /// Start the RTCM streaming to the local redirect server from the given NTRIP address.
    fn set_rtcm_input_mode(&mut self, username: &str, password: &str, server: &str, mount_point: &str, port: u16) {
        
        log::info!("Setting the GPS to accept RTCM input.");
        let address_in = "ntrip://".to_string()+username+":"+password+"@"+server+":"+&port.to_string()+"/"+&mount_point.to_string();
        let address_out = "tcpcli://127.0.0.1:".to_string()+&self.io_port.to_string();

        log::info! ("Command: str2str -in {} -out {}", address_in, address_out);

        //Kill the current runner before restarting, if running.
        if let Some(mut cmd) = self.ntrip_command.take() {
            cmd.kill().expect("str2str couldn't be killed!");
        };

        self.ntrip_command = Some(Command::new ("str2str").arg("-in").arg(address_in).arg("-out").arg(address_out).spawn().expect("Failed to execute str2str."));
        
        let ubx_commands = vec![
                            ("-z", "CFG-UART2-ENABLED,1"),
                            ("-z", "CFG-UART2-BAUDRATE,115200"), //ship mode
                            ("-z", "CFG-UART2INPROT-RTCM3X,1")
                        ];
        GPSControl::run_ubx_commands(ubx_commands);
    }

    fn set_rover_mode(&self) {
        log::info!("Setting the GPS into rover mode, and the serial TX to send out NMEA data.");
        let _output = Command::new ("gpsctl").arg("-s").arg(GPS_BAUDRATE).output().expect("Failed to execute gpsctl.");
        log::info!("Baudrate set to {}.", GPS_BAUDRATE);

        let ubx_commands = vec![
                            ("-p", "RESET"),
                            ("-e", "NMEA"),
                            ("-d", "BINARY"),
                            ("-e", "GLONASS"),
                            ("-d", "BEIDOU"),
                            ("-d", "GALILEO"),
                            ("-d", "SBAS"),
                            ("-e", "GPS"),
                            ("-d", "RAWX"),
                            ("-z", "CFG-NMEA-HIGHPREC,1"),
                            ("-z", "CFG-NAVSPG-DYNMODEL,0"), //ship mode
                            ("-z", "CFG-UART2-ENABLED,1"),
                            ("-z", "CFG-UART2-BAUDRATE,115200"),
                            ("-z", "CFG-UART2OUTPROT-NMEA,1")
                            //("-z", "CFG-MSGOUT-UBX_RXM_RAWX_USB,1")
                        ];
        GPSControl::run_ubx_commands(ubx_commands);
    }

    fn run_ubx_commands(commands: Vec<(&str, &str)>) {
        for (flag, configuration) in commands {
            let output = Command::new("ubxtool").arg("-P").arg(UBLOX_VERSION).arg(flag).arg(configuration).output().expect("ubxtool failed  to run");
            let ubx_output = String::from_utf8_lossy(&output.stdout);
            if ubx_output.contains(UBX_ACK) {
                log::info!("ubxtool successfully ran with {} {} flags.", flag, configuration);
            } else {
                log::info!("ubxtool failed with {} {} flags.", flag, configuration);
            }
        }
    }
}

impl Actor for GPSControl {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        let socket_addr = SocketAddr::new(self.ip_address, self.port);

        //Start the gpsd daemone if it's not already running.
        if !port_scanner::scan_port_addr(&socket_addr) {
            log::info!("Starting GPSD Service.");
            self.gpsd_command = Some(Command::new("gpsd")
                                .arg(self.gps_usb_port.clone())
                                .arg("-N")
                                .spawn()
                                .expect("The GPSD daemon failed to start."));
            std::thread::sleep(std::time::Duration::from_secs(6));
        }

        self.set_rover_mode();
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        if let Some(mut cmd) = self.ntrip_command.take() {
            cmd.kill().expect("str2str couldn't be killed!");
        };
        if let Some(mut cmd) = self.gpsd_command.take() {
            cmd.kill().expect("gpsd couldn't be killed!");
        };
        if let Some(mut cmd) = self.rinex_collection_command.take() {
            cmd.kill().expect("gpsrinex couldn't be killed!");
        }
    }
}

/// Implement handlers for various actor messages.
impl Handler<GPSMode> for GPSControl {
    type Result = Result<(), Box<dyn std::error::Error + Send + Sync>>;

    fn handle(&mut self, msg: GPSMode, ctx: &mut Context<Self>) -> Self::Result {
        log::info!("Handling set GPS mode in GPS control: {:?}", msg);
        match msg {
            GPSMode::Base(username, password, server, mount_point, port) => {
                self.set_base_station_mode(&username, &password, &server, &mount_point, port);
                //todo!( "Send set rover to LORA stack and other device." );
            },
            GPSMode::Standalone => {
                self.set_rover_mode();
                //todo! ( "Send set base to LORA stack and other device." );
            },
            GPSMode::RAW(filename, interval, number_of_collections) => { 
                self.set_raw_mode(&filename, interval, number_of_collections);
            }
            GPSMode::RtcmIn(username, password, server, mount_point, port) => {
                self.set_rover_mode();
                self.set_rtcm_input_mode(&username, &password, &server, &mount_point, port);
            },
            GPSMode::Stopped => {
                ctx.stop();
            }
        }
        Ok(())
    }
}