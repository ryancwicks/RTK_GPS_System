use actix_web::{Error, HttpResponse, Responder, get, post, web};
use actix_files::NamedFile;
use serde::Deserialize;
use crate::gps_interface::gps_control::{GPSControl, GPSMode};
use crate::web_socket::GPSWebSocketMonitor;
use crate::settings::{Modes, SettingsMessage, SettingsHandler};
use actix::prelude::*;

type WebData = web::Data<(Addr<GPSWebSocketMonitor>, Addr<GPSControl>, Addr<SettingsHandler>)>;

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

#[post("/start")]
async fn start(data: WebData, info: web::Json<RTKClientConfig>) -> impl Responder {
    log::info!("Starting the GPS system.");
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

#[get("/settings")]
async fn get_settings(data: WebData) -> impl Responder { 
    log::info!("Handling get settings api command.");
    let settings_manager = &data.get_ref().2;

    let message = SettingsMessage::GetSettings();
    let settings_return_future = settings_manager.send(message).await;

    let settings_return = settings_return_future.expect("Failed to get settings from settings manager");
    web::Json(settings_return.expect("Received settings error from settings manager."))
}

#[post("/settings")]
async fn set_settings(data: WebData, info: web::Json<Modes>) -> impl Responder {
    log::info!("Handling set settings api command");
    let settings_manager = &data.get_ref().2;
    let settings = info.0;

    let message = SettingsMessage::SetSettings(settings);
    let settings_return_future = settings_manager.send(message).await;

    let settings_return = settings_return_future.expect("Failed to set settings in settings manager.");
    settings_return.expect("Received setting erro from settings manager.");
    
    HttpResponse::Ok().finish()
}

#[get("/shutdown")]
pub async fn shutdown(data: WebData) -> Result<HttpResponse, Error> {//(processor: web::Data<Addr<Processor>>) -> Result<HttpResponse, Error> {
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


