use std::{
    env::{self, temp_dir},
    fs::{self, File},
    io::{Error, copy},
    path::PathBuf,
};

use flate2::bufread::GzDecoder;
use reqwest::blocking::get;

pub fn install(loc: &str) -> std::io::Result<()> {
    let path = PathBuf::from(loc);
    if path.exists() && path.is_file() {
        install_archive(&path)
    } else {
        download_install_archive(loc)
    }
}

pub fn install_archive(path: &PathBuf) -> std::io::Result<()> {
    match path.extension() {
        Some(ext) if ext == "zip" => install_from_zip(path),
        Some(ext) if ext == "tar" => install_from_tar(path),
        Some(ext) if ext == "gz" || ext == "tgz" => install_from_tar_gz(path),
        Some(ext) => {
            let error_msg = format!("Unsupported file type for installation: {:?}", ext);
            Err(Error::new(std::io::ErrorKind::InvalidInput, error_msg))
        }
        None => {
            let error_msg =
                "No file extension found. Please provide a valid archive file (zip, tar, tar.gz).";
            Err(Error::new(std::io::ErrorKind::InvalidInput, error_msg))
        }
    }
}

pub fn install_from_zip(path: &PathBuf) -> std::io::Result<()> {
    // Placeholder for actual zip extraction logic
    let mut archive = zip::ZipArchive::new(File::open(path)?)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => path,
            None => continue,
        };

        {
            let comment = file.comment();
            if !comment.is_empty() {
                println!("File {i} comment: {comment}");
            }
        }

        if file.is_dir() {
            println!("File {} extracted to \"{}\"", i, outpath.display());
            fs::create_dir_all(&outpath).unwrap();
        } else {
            println!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            copy(&mut file, &mut outfile).unwrap();
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }
    println!("Extracting zip archive at {:?}", path);
    Ok(())
}

pub fn install_from_tar(path: &PathBuf) -> std::io::Result<()> {
    let file = File::open(path)?;
    let mut archive = tar::Archive::new(file);
    let current_exe_path = env::current_exe().map_err(Error::other)?;
    let parent_dir = current_exe_path.parent().ok_or_else(|| {
        Error::new(
            std::io::ErrorKind::NotFound,
            "Current executable path has no parent",
        )
    })?;
    archive.unpack(parent_dir)
}

pub fn install_from_tar_gz(path: &PathBuf) -> std::io::Result<()> {
    let file = File::open(path)?;
    let file = std::io::BufReader::new(file);
    let decompresed = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decompresed);
    // get exectuable path
    let current_exe_path = env::current_exe().map_err(Error::other)?;
    let parent_dir = current_exe_path.parent().ok_or_else(|| {
        Error::new(
            std::io::ErrorKind::NotFound,
            "Current executable path has no parent",
        )
    })?;

    archive.unpack(parent_dir)
}

pub fn download_archive(url: &str) -> std::io::Result<PathBuf> {
    let temp_file = temp_dir().join("downloaded_archive");
    let mut file = File::create(&temp_file).map_err(Error::other)?;
    let response = get(url).map_err(Error::other)?;
    if !response.status().is_success() {
        return Err(Error::other(format!(
            "Failed to download file: {}",
            response.status()
        )));
    }
    let content = response
        .bytes()
        .map_err(|e| Error::other(format!("Failed to read response: {}", e)))?;
    copy(&mut content.as_ref(), &mut file).map_err(Error::other)?;
    Ok(temp_file)
}

pub fn download_install_archive(url: &str) -> std::io::Result<()> {
    let download_result = download_archive(url)?;
    install_archive(&download_result)
}
