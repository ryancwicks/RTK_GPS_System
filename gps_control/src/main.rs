use actix::prelude::*;
use actix_files::Files;
use tokio::sync::{broadcast, mpsc};
use clap::Parser;

mod web_socket;
mod api;
mod gps_interface;
mod lora_streaming;
mod settings;

use settings::{Cli, Modes, SettingsHandler};
use port_redirector::input_stream::InputSocket;
use port_redirector::retransmit_server::RetransmitServer;
use gps_interface::gps_control::{GPS_DATA_DIR, GPSMode};

const STATIC_FILES: &str= "./static";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    pretty_env_logger::init();
    log::info!("Starting UBlox GPS Control Software.");

    std::fs::create_dir_all(GPS_DATA_DIR)?;
    let cli = Cli::parse();

    //Setup the settings handler
    let settings_handler = SettingsHandler::new(cli.mode.clone()).start();

    //Setup the serial port redirector
    let input_serial_port = InputSocket::Serial {port_name: cli.gps_tty_port, baudrate: Some(115200), rd: None, tx: None};
    
    //Broadcast port for reading in data on the input port and outputting it on all broadcast channels
    let (broadcast_from_input, _) = broadcast::channel(32);
    let broadcast_from_input_1 = broadcast_from_input.clone();

    //Mutiple producers to read data in from the server ports and output on the single output port.
    let (tx_to_input, rx_to_input) = mpsc::channel(32);

    //open the socket and start the reading process.
    let mut socket_reader = InputSocket::connect(input_serial_port).await?;
    tokio::spawn( async move { socket_reader.run_loop(broadcast_from_input, rx_to_input).await; });

    // Set up server.
    let retransmit_server = RetransmitServer::new(cli.output_port, tx_to_input, broadcast_from_input_1).await?;
    tokio::spawn( async move { retransmit_server.run_loop().await; });
    
    let socket_monitor = web_socket::GPSWebSocketMonitor::new().start();
    let gps_control = gps_interface::gps_control::GPSControl::new(Some(&cli.gpsd_server), Some(cli.gpsd_port), Some(cli.gps_usb_port), Some(cli.output_port)).start();
    
    let mut gps_interface = gps_interface::gps_interface::GPSInterface::new(Some(&cli.gpsd_server), Some(cli.gpsd_port), socket_monitor.clone());

    tokio::spawn( async move {
        gps_interface.run_handler().await;
    });

    if cli.start {
        match cli.mode {
            Modes::RTKRover{username, password, server, mount_point, port} => {
                gps_control.do_send(GPSMode::RtcmIn(username, password, server, mount_point, port));
            },
            Modes::RTKBase{username, password, server, mount_point, port} => {
                gps_control.do_send(GPSMode::Base(username, password, server, mount_point, port));
            },
            Modes::PPPMode{filename, interval, number_of_collections} => {
                gps_control.do_send(GPSMode::RAW(filename, interval, number_of_collections));
            },
            Modes::Standalone => {
                gps_control.do_send(GPSMode::Standalone);
            }
        };
    }

    use actix_web::{middleware, web, App, HttpServer};
    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new((socket_monitor.clone(), gps_control.clone(), settings_handler.clone())))
            // enable logger
            .wrap(middleware::Logger::default())
            // serve static files
            .service(web::resource("/").route(web::get().to(api::index)))
            .service(Files::new("/static", &STATIC_FILES).index_file("index.html"))
            .service(Files::new("/data/", GPS_DATA_DIR))
            // websocket route
            .service(web::resource("/api/subscribe").route(web::get().to(web_socket::ws_index)))
            // rest API
            .service(web::scope("/api")
                        .service(api::start)
                        .service(api::get_settings)
                        .service(api::set_settings)
                        .service(api::shutdown))
            
    })
    .bind(("0.0.0.0", cli.web_port))?
    .run()
    .await    
}
