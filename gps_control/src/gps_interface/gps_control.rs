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
    Base(String /*username*/, String/*password*/, String/*server*/, String/*mount_point*/, u16/*port*/, u32 /*survey_dwell_time*/, u32/*survey_position_accuracy*/, Option<f64>/*fixed_ecef_x*/, Option<f64>/*fixed_ecef_y*/, Option<f64>/*fixed_ecef_z*/, Option<f64>/*fixed_ecef_accuracy*/),
    Standalone,
    RAW (String /*data_directory*/,  String /*filename*/, u32 /*interval*/, u32 /*number_of_collections*/),
    RtcmIn(String /*username*/, String/*password*/, String/*server*/, String/*mount_point*/, u16/*port*/),
    Stopped,
}


///GPS control strucutre, used to set up the the gpsd server through a combination of command line gpsctl and ubxtool commands.
pub struct GPSControl {
    ip_address: IpAddr,
    port: u16,
    gpsd_command: Option<std::process::Child>,
    ntrip_command: Option<std::process::Child>,
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
            rinex_collection_command: None,
            gps_usb_port: gps_usb_port,
            io_port: io_port
        }    
    }

    ///Start collecting data in raw mode, saving the ubx data to a file.
    ///  - data_directory: directory to save the data to.
    ///  - filename: filename to save the rinex observations to.
    ///  - interval_in_s: number of seconds to wait between collections.
    ///  - duration_in_min: number of minutes to collect data for.
    fn set_raw_mode(&mut self, data_directory: &str, filename: &str, interval_in_s: u32, number_of_collections: u32) { 
        log::info!("Setting GPS into raw binary mode.");

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

        let filename = std::path::Path::new(data_directory).join(filename);
        self.rinex_collection_command = Some(Command::new ("gpsrinex")
                                .arg("-i").arg(interval_in_s.to_string())
                                .arg("-n").arg(number_of_collections.to_string())
                                .arg("-f").arg(filename)
                                .spawn().expect("Failed to execute gps2rinex."));
    
    }

    /// Set the rover into base station mode, enabling appropriate RTCM outputs and position modes.
    /// This is then followed by setting up the str2str process to stream those rtcm corrections.
    fn set_base_station_mode(&mut self, username: &str, password: &str, server: &str, mount_point: &str, port: u16,
                             survey_dwell_time: u32, survey_position_accuracy: u32, 
                             fixed_ecef_x: Option<f64>, fixed_ecef_y: Option<f64>, fixed_ecef_z: Option<f64>, fixed_ecef_accuracy: Option<f64>) {
        log::info!("Setting the GPS into base station mode and setting the serial port TX to output RTCM messages.");

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
                            ("-z", "CFG-UART2-ENABLED,1"),
                            ("-z", "CFG-UART2-BAUDRATE,115200"),
                            ("-z", "CFG-NAVSPG-DYNMODEL,2"), //Stationary mode
                            ("-z", "CFG-UART2OUTPROT-NMEA,0"),
                            ("-z", "CFG-UART2OUTPROT-RTCM3X,1"),
                        ];
        GPSControl::run_ubx_commands(ubx_commands);

        if fixed_ecef_x != None && fixed_ecef_y != None && fixed_ecef_z != None && fixed_ecef_accuracy != None {
            let split_position = |val: f64| -> (i64, i64) {
                let std_precision: i64 = val.floor() as i64;
                let high_precision: i64 = ((val - val.floor()) * 100.0).floor() as i64;
                (std_precision, high_precision)
            };
            
            let (ecef_x_sp, ecef_x_hp) = split_position(fixed_ecef_x.unwrap());
            let (ecef_y_sp, ecef_y_hp) = split_position(fixed_ecef_y.unwrap());
            let (ecef_z_sp, ecef_z_hp) = split_position(fixed_ecef_z.unwrap());
            let ecef_acc = fixed_ecef_accuracy.unwrap();

            let str_ecef_x = "CFG-TMODE-ecef_X,".to_owned() + &ecef_x_sp.to_string();
            let str_ecef_x_hp = "CFG-TMODE-ecef_X_HP,".to_owned() + &ecef_x_hp.to_string();
            let str_ecef_y = "CFG-TMODE-ecef_Y,".to_owned() + &ecef_y_sp.to_string();
            let str_ecef_y_hp = "CFG-TMODE-ecef_Y_HP,".to_owned() + &ecef_y_hp.to_string();
            let str_ecef_z = "CFG-TMODE-ecef_Z,".to_owned() + &ecef_z_sp.to_string();
            let str_ecef_z_hp = "CFG-TMODE-ecef_Z_HP,".to_owned() + &ecef_z_hp.to_string();
            let str_ecef_acc = "CFG-TMODE-FIXED_POS_ACC,".to_owned() + &ecef_acc.to_string();         
            
            let ubx_commands = vec![
                            ("-z", "CFG-TMODE-MODE,2"), //fixed base mode
                            ("-z", "CFG-TMODE-POS_TYPE,0"), //ecef co-ordinates
                            ("-z", &str_ecef_x),
                            ("-z", &str_ecef_x_hp),
                            ("-z", &str_ecef_y),
                            ("-z", &str_ecef_y_hp),
                            ("-z", &str_ecef_z),
                            ("-z", &str_ecef_z_hp),
                            ("-z", &str_ecef_acc),
                        ];
            GPSControl::run_ubx_commands(ubx_commands);
        } else {
            let min_dur_setting = "CFG-TMODE-SVIN_MIN_DUR,".to_owned() + &survey_dwell_time.to_string();
            let min_acc_setting = "CFG-TMODE-SVIN_ACC_LIMIT,".to_owned() + &survey_position_accuracy.to_string();
            let ubx_commands = vec![
                            ("-z", "CFG-TMODE-MODE,1"), //Survey in mode
                            ("-z", &min_dur_setting),
                            ("-z", &min_acc_setting),
                        ];
            GPSControl::run_ubx_commands(ubx_commands);
        }


        log::info!("Starting the ntrip caster.");
        let address_out = "ntrip://".to_string()+username+":"+password+"@"+server+":"+&port.to_string()+"/"+&mount_point.to_string();
        let address_in = "tcpcli://127.0.0.1:".to_string()+&self.io_port.to_string();

        log::info! ("Command: str2str -in {} -out {}", address_in, address_out);

        //Kill the current runner before restarting, if running.
        if let Some(mut cmd) = self.ntrip_command.take() {
            cmd.kill().expect("str2str couldn't be killed!");
        };

        self.ntrip_command = Some(Command::new ("str2str").arg("-in").arg(address_in).arg("-out").arg(address_out).spawn().expect("Failed to execute str2str."));
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
                            ("-z", "CFG-UART2INPROT-RTCM3X,1")
                        ];
        GPSControl::run_ubx_commands(ubx_commands);
    }

    fn set_rover_mode(&self) {
        log::info!("Setting the GPS into rover mode, and the serial TX to send out NMEA data.");

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
                            ("-z", "CFG-UART2OUTPROT-NMEA,1"),
                            ("-z", "CFG-MSGOUT-NMEA_ID_ZDA_UART2,1"), //set ZDA output to 1 Hz.
                            ("-z", "CFG-UART2OUTPROT-RTCM3X,0")
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
            let _output = Command::new ("gpsctl").arg("-s").arg(GPS_BAUDRATE).output().expect("Failed to execute gpsctl.");
            log::info!("Baudrate set to {}.", GPS_BAUDRATE);
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
            GPSMode::Base(username, password, server, mount_point, port,
                          survey_dwell_time, survey_position_accuracy, 
                          fixed_ecef_x, fixed_ecef_y, fixed_ecef_z, fixed_ecef_accuracy) => {
                self.set_base_station_mode(&username, &password, &server, &mount_point, port,
                                           survey_dwell_time, survey_position_accuracy, 
                                           fixed_ecef_x, fixed_ecef_y, fixed_ecef_z, fixed_ecef_accuracy);
                //todo!( "Send set rover to LORA stack and other device." );
            },
            GPSMode::Standalone => {
                self.set_rover_mode();
                //todo! ( "Send set base to LORA stack and other device." );
            },
            GPSMode::RAW(data_directory, filename, interval, number_of_collections) => { 
                self.set_raw_mode(&data_directory, &filename, interval, number_of_collections);
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