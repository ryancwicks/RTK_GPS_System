use actix::prelude::*;
use actix::Message;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use port_scanner;
use std::process::Command;

const GPS_BAUDRATE: &str = "115200";
const UBLOX_VERSION: &str = "27.30";
const UBX_ACK: &str = "UBX-ACK-ACK:";

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
    Base,
    Rover,
    RtcmIn (/*username*/ String, /*password*/ String, /*server*/ String, /*mount_point*/ String, /*port*/ u16),
    Stopped,
}


///GPS control strucutre, used to set up the the gpsd server through a combination of command line gpsctl and ubxtool commands.
pub struct GPSControl {
    pub ip_address: IpAddr,
    pub port: u16,
    gpsd_command: Option<std::process::Child>,
    ntrip_command: Option<std::process::Child>,
    nmea_publish_command: Option<std::process::Child>,
}

impl GPSControl {
    pub fn new (ip_address: Option<&str>, 
                port: Option<u16>  ) -> Self {

        let ip_address = match ip_address {
            Some(ip) => ip,
            None => "127.0.0.1"
        };
        let port = match port {
            Some(p) => p,
            None => 2947
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
        }    
    }

    fn set_raw_mode(&self) {

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
        for (flag, configuration) in ubx_commands {
            let output = Command::new("ubxtool").arg("-P").arg(UBLOX_VERSION).arg(flag).arg(configuration).output().expect("ubxtool failed  to run");
            let ubx_output = String::from_utf8_lossy(&output.stdout);
            if ubx_output.contains(UBX_ACK) {
                log::info!("ubxtool successfully ran with {} {} flags.", flag, configuration);
            } else {
                log::info!("ubxtool failed with {} {} flags.", flag, configuration);
            }


        }
        
    }

    fn set_base_station_mode(&self) -> Result<(), std::io::Error> {
        log::info!("Setting the GPS into base station mode and setting the serial port TX to output RTCM messages.");
        let _output = Command::new ("gpsctl").arg("-s").arg(GPS_BAUDRATE).output().expect("Failed to execute gpsctl.");
        log::info!("Baudrate set to {}.", GPS_BAUDRATE)
;            
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
        for (flag, configuration) in ubx_commands {
            let output = Command::new("ubxtool").arg("-P").arg(UBLOX_VERSION).arg(flag).arg(configuration).output().expect("ubxtool failed  to run");
            let ubx_output = String::from_utf8_lossy(&output.stdout);
            if ubx_output.contains(UBX_ACK) {
                log::info!("ubxtool successfully ran with {} {} flags.", flag, configuration);
            } else {
                log::info!("ubxtool failed with {} {} flags.", flag, configuration);
            }
        }
        Ok(())
    }

    fn set_rtcm_input_mode(&mut self, username: &str, password: &str, server: &str, mount_point: &str, port: u16) {
        log::info!("Setting the GPS to accept RTCM input on serial port 2 at 115200.");
        let address_in = "ntrip://".to_string()+username+":"+password+"@"+server+":"+&port.to_string()+"/"+&mount_point.to_string();
        let address_out = "serial://ttyUSB0:115200";

        log::info! ("Command: str2str -in {} -out {}", address_in, address_out);

        //Only start this process once.
        match self.ntrip_command {
            Some(_) => {},
            None => {
                self.ntrip_command = Some(Command::new ("str2str").arg("-in").arg(address_in).arg("-out").arg(address_out).spawn().expect("Failed to execute str2str."));
            }
        }

        let ubx_commands = vec![
                            ("-z", "CFG-UART2-ENABLED,1"),
                            ("-z", "CFG-UART2-BAUDRATE,115200"), //ship mode
                            ("-z", "CFG-UART2INPROT-RTCM3X,1")
                        ];
        for (flag, configuration) in ubx_commands {
            let output = Command::new("ubxtool").arg("-P").arg(UBLOX_VERSION).arg(flag).arg(configuration).output().expect("ubxtool failed  to run");
            let ubx_output = String::from_utf8_lossy(&output.stdout);
            if ubx_output.contains(UBX_ACK) {
                log::info!("ubxtool successfully ran with {} {} flags.", flag, configuration);
            } else {
                log::info!("ubxtool failed with {} {} flags.", flag, configuration);
            }
        }
    }

    fn set_rover_mode(&self) {
        log::info!("Setting the GPS into rover mode, and the serial TX to send out NMEA data.");
        let _output = Command::new ("gpsctl").arg("-s").arg(GPS_BAUDRATE).output().expect("Failed to execute gpsctl.");
        log::info!("Baudrate set to {}.", GPS_BAUDRATE)
;            
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
                            //("-z", "CFG-MSGOUT-UBX_RXM_RAWX_USB,1")
                        ];
        for (flag, configuration) in ubx_commands {
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
                                .arg("/dev/ttyACM0")
                                .arg("-N")
                                .spawn()
                                .expect("The GPSD daemon failed to start."));
            std::thread::sleep(std::time::Duration::from_secs(6));
        }

        self.set_rover_mode();
        //strself.set_rtcm_input_mode("ryan@voyis.com", "none", "rtk2go.com", "AVRIL", 2101);
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        if let Some(mut cmd) = self.ntrip_command.take() {
            cmd.kill().expect("str2str couldn't be killed!");
        };
        if let Some(mut cmd) = self.gpsd_command.take() {
            cmd.kill().expect("gpsd couldn't be killed!");
        };

    }

}

/// Implement handlers for various actor messages.
impl Handler<GPSMode> for GPSControl {
    type Result = Result<(), Box<dyn std::error::Error + Send + Sync>>;

    fn handle(&mut self, msg: GPSMode, ctx: &mut Context<Self>) -> Self::Result {
        log::info!("Handling set GPS mode in GPS control: {:?}", msg);
        match msg {
            GPSMode::Base => {
                self.set_base_station_mode()?;
                //todo!( "Send set rover to LORA stack and other device." );
            },
            GPSMode::Rover => {
                self.set_rover_mode();
                //todo! ( "Send set base to LORA stack and other device." );
            }
            GPSMode::RtcmIn(username, password, server, mount_point, port) => {
                self.set_rtcm_input_mode(&username, &password, &server, &mount_point, port);
            }
            GPSMode::Stopped => {
                ctx.stop();
            }
        }
        Ok(())
    }
}