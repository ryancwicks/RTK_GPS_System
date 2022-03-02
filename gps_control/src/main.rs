use actix::prelude::*;
use actix_files::Files;

mod web_socket;
mod api;
mod gps_interface;
mod lora_streaming;

const STATIC_FILES: &str= "./static";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    pretty_env_logger::init();
    log::info!("Starting UBlox GPS Control Software.");
    
    let socket_monitor = web_socket::GPSWebSocketMonitor::new().start();
    let _gps_control = gps_interface::GPSControl::new(Some("127.0.0.1"), Some(2947)).start();
    
    let mut gps_interface = gps_interface::GPSInterface::new(Some("127.0.0.1"), Some(2947), socket_monitor.clone());

    tokio::spawn( async move {
        gps_interface.run_handler().await;
    });

    use actix_web::{middleware, web, App, HttpServer};
    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(socket_monitor.clone()))
            // enable logger
            .wrap(middleware::Logger::default())
            // serve static files
            .service(web::resource("/").route(web::get().to(api::index)))
            .service(Files::new("/static", &STATIC_FILES).index_file("index.html"))
            // websocket route
            .service(web::resource("/api/subscribe").route(web::get().to(web_socket::ws_index)))
            // rest API
            .service(web::scope("/api")
                        .service(api::shutdown))
            
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await


    
}
