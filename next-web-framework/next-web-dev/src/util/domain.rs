use std::{io, path::PathBuf};


pub async fn ready(s: impl AsRef<str>) -> io::Result<()> {
    let path = PathBuf::from(s.as_ref());
    tokio::fs::remove_file(&path).await?;

    tokio::fs::create_dir_all(path.parent().unwrap())
            .await?;
           
    Ok(())
}