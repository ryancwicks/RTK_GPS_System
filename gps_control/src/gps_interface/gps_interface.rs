
use futures::prelude::*;
use gpsd_proto::UnifiedResponse;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;
use tokio_util::codec::LinesCodec;
use serde::Serialize;
use actix::prelude::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::web_socket;

#[derive(Serialize, Clone, Debug)]
pub struct GPSData {
    device_path: String,
    driver: String,
    activated: String,

    mode: String,
    lat: f64,
    lon: f64,
    alt: f32,
    track: f32,
    speed: f32,
    time: String,
    rms: f32,
    orient: f32,
    major: f32,
    minor: f32,
}

///The GPSD interface. Runs asynchronously from the GPSD server.
pub struct GPSInterface {
    ip_address: IpAddr,
    port: u16,
    web_socket_monitor: Addr<web_socket::GPSWebSocketMonitor>,

    gps: GPSData,
}

impl GPSInterface {

    /// Generate an empty GPSD interface.
    pub fn new (ip_address: Option<&str>, 
                port: Option<u16>, 
                web_socket_monitor: Addr<web_socket::GPSWebSocketMonitor>) -> Self {
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

        let gps = GPSData {
            device_path: "".to_string(),
            driver: "".to_string(),
            activated: "".to_string(),

            mode: "".to_string(),
            lat: 0.,
            lon: 0.,
            alt: 0.,
            track: 0.,
            speed: 0.,
            time: "".to_string(),
            rms: 0.,
            orient: 0.,
            major: 0.,
            minor: 0.,
        };

        GPSInterface{
            ip_address: ip_address,
            port: port,
            web_socket_monitor: web_socket_monitor,

            gps: gps,
        }
    }

    /// Start the handler for reading in GPS data. 
    /// This process will start the docker instance of the gpsd daemon if it isn't already running.
    pub async fn run_handler(self: &mut Self) {

        let socket_addr = SocketAddr::new(self.ip_address, self.port);

        let socket = TcpStream::connect(&socket_addr).await.expect( &format!("Failed to connect to the GPSD daemon at ip {}:{}", self.ip_address, self.port) );

        let mut framed = Framed::new(socket, LinesCodec::new());

        framed.send(gpsd_proto::ENABLE_WATCH_CMD.to_string()).await.expect("Failed to get expected response from GPSD.");

        while let Some(Ok(line)) = framed.next().await {

            match serde_json::from_str(&line) {
                Ok(rd) => match rd {
                    UnifiedResponse::Version(v) => {
                        if v.proto_major < gpsd_proto::PROTO_MAJOR_MIN {
                            panic!("Gpsd major version mismatch");
                        }
                        log::info!("Gpsd version {} connected", v.rev);
                    }
                    UnifiedResponse::Devices(_) => {}
                    UnifiedResponse::Watch(_) => {}
                    UnifiedResponse::Device(d) => {
                        //log::debug!("Device {:?}", d);
                        self.gps.device_path = d.path.unwrap_or("".to_string());
                        self.gps.driver = d.driver.unwrap_or("".to_string());
                        self.gps.activated = d.activated.unwrap_or("".to_string());
                    },
                    UnifiedResponse::Tpv(t) => {
                        //log::debug!("Tpv {:?}", t);
                        self.gps.mode = t.mode.to_string();
                        self.gps.lat = t.lat.unwrap_or(0.0);
                        self.gps.lon = t.lon.unwrap_or(0.0);
                        self.gps.alt = t.alt.unwrap_or(0.0);
                        self.gps.track = t.track.unwrap_or(0.0);
                        self.gps.speed = t.speed.unwrap_or(0.0);
                        self.gps.time = t.time.unwrap_or("".to_string());
                    },
                    UnifiedResponse::Sky(_s) => {}//log::debug!("Sky {:?}", s),
                    UnifiedResponse::Pps(p) => log::debug!("PPS {:?}", p),
                    UnifiedResponse::Gst(g) => {
                        //log::debug!("GST {:?}", g);
                        //g.device.unwrap_or("".to_string());
                        self.gps.time = g.time.unwrap_or("".to_string());
                        self.gps.rms = g.rms.unwrap_or(0.);
                        self.gps.major = g.major.unwrap_or(0.);
                        self.gps.minor = g.minor.unwrap_or(0.); 
                        self.gps.orient = g.orient.unwrap_or(0.);
                        self.gps.lat = g.lat.unwrap_or(0.).into(); 
                        self.gps.lon = g.lon.unwrap_or(0.).into();
                        self.gps.alt = g.alt.unwrap_or(0.);
                    },
                    //need to add RAW support to gpsd_proto
                },
                Err(_e) => {
                    //log::error!("Error decoding: {}", e);
                }
            };

            let gps_event = web_socket::GPSEvent {data: self.gps.clone()};
            self.web_socket_monitor.do_send(gps_event);
        }

    }

}




