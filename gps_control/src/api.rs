use actix_web::{Error, HttpResponse, get};
use actix_files::NamedFile;

pub async fn index() -> Result<NamedFile, Error> {
    let file = NamedFile::open_async("./static/index.html").await?;
    Ok(file)
}

#[get("/shutdown")]
pub async fn shutdown() -> Result<HttpResponse, Error> {//(processor: web::Data<Addr<Processor>>) -> Result<HttpResponse, Error> {
    //processor.do_send(SetState::Shutdown);
    log::info!("Shutting down the server.");
    tokio::spawn( async move {
        std::thread::sleep(std::time::Duration::from_secs(2));
        log::info!("Shutting down.");
        std::process::exit(0);
    } );
    Ok(HttpResponse::Ok()
    .content_type("text/plain")
    .body("Shutting down!"))
}


