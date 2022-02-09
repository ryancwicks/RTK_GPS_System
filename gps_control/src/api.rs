use actix_web::{Error};
use actix_files::NamedFile;

pub async fn index() -> Result<NamedFile, Error> {
    let file = NamedFile::open_async("./static/index.html").await?;
    Ok(file)
}


