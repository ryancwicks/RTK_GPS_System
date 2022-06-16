use actix_web::{Error, HttpResponse, Responder, get, post, web};
use actix_files::NamedFile;
use serde::Deserialize;
use crate::gps_interface::gps_control::{GPSControl, GPSMode};
use crate::web_socket::GPSWebSocketMonitor;
use actix::prelude::*;

pub async fn index() -> Result<NamedFile, Error> {
    let file = NamedFile::open_async("./static/index.html").await?;
    Ok(file)
}

#[derive(Deserialize)]
struct RTKClientConfig {
    username: String,
    password: String,
    server: String,
    mount_point: String,
    port: u16,
}

#[post("/start_rtk")]
async fn start_rtk(data: web::Data<(Addr<GPSWebSocketMonitor>, Addr<GPSControl>)>, info: web::Json<RTKClientConfig>) -> impl Responder {
    log::info!("Starting RTK input mode .");
    let gps_control = &data.get_ref().1;
    
    let rtcm_mode = GPSMode::RtcmIn(info.username.clone(), info.password.clone(), info.server.clone(), info.mount_point.clone(), info.port);
    let control_future = gps_control.send(rtcm_mode).await;
    
    match control_future {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => {
            log::error!("Failed to set the RTK input mode: {}", e);
            HttpResponse::InternalServerError()
        }
    }
}

#[get("/shutdown")]
pub async fn shutdown(data: web::Data<(Addr<GPSWebSocketMonitor>, Addr<GPSControl>)>) -> Result<HttpResponse, Error> {//(processor: web::Data<Addr<Processor>>) -> Result<HttpResponse, Error> {
    //processor.do_send(SetState::Shutdown);
    log::info!("Shutting down the server.");
    tokio::spawn( async move {
        std::thread::sleep(std::time::Duration::from_secs(2));
        log::info!("Shutting down.");
        std::process::exit(0);
    } );

    let gps_control = &data.get_ref().1;
    let control_future = gps_control.send(GPSMode::Stopped).await;
    match control_future {
        Ok(_) => (),
        Err(e) => {
            log::error!("Failed to stop the GPS service cleanly: {}", e)
        }
    };

    Ok(HttpResponse::Ok()
    .content_type("text/plain")
    .body("Shutting down!"))
}


