use std::{
    env::{self, temp_dir},
    fs::{self, File},
    io::{self, Error, Read, Write, copy},
    path::PathBuf,
};

use flate2::bufread::GzDecoder;
use reqwest::blocking::get;

use crate::{
    STDOUT_WRITE,
    print::{p_error, p_good},
};

pub fn install(loc: &str) -> io::Result<()> {
    let path = PathBuf::from(loc);
    if path.exists() && path.is_file() {
        install_archive(&path)
    } else if reqwest::Url::parse(loc).is_ok() {
        download_install_archive(loc)
    } else {
        Err(Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Location at '{}' appears to be an invalid URL and not exist locally.",
                loc
            ),
        ))
    }
}

pub fn install_archive(path: &PathBuf) -> io::Result<()> {
    match path.extension() {
        Some(ext) if ext == "zip" => install_from_zip(path),
        Some(ext) if ext == "tar" => install_from_tar(path),
        Some(ext) if ext == "gz" || ext == "tgz" => install_from_tar_gz(path),
        _ => install_simple_file(path),
    }
}

pub fn install_from_zip(path: &PathBuf) -> io::Result<()> {
    // Placeholder for actual zip extraction logic
    eprintln!("Installing from ZIP archive: {:?}", path);
    let mut archive = zip::ZipArchive::new(File::open(path)?)?;
    let archive_len = archive.len();
    let pb = if unsafe { STDOUT_WRITE } {
        indicatif::ProgressBar::new(archive_len as u64).with_style(
            indicatif::ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg} ({elapsed_precise})")
                .unwrap(),
        )
    } else {
        indicatif::ProgressBar::hidden()
    };
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => path,
            None => continue,
        };
        pb.set_message(format!(
            "Extracting {} ({}/{})",
            outpath.display(),
            i + 1,
            archive_len
        ));

        if file.is_dir() {
            fs::create_dir_all(&outpath).unwrap();
        } else {
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
    let fname = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            Error::new(
                io::ErrorKind::InvalidInput,
                "Provided path has no valid file name",
            )
        })?
        .strip_suffix(".zip")
        .ok_or_else(|| {
            Error::new(
                io::ErrorKind::InvalidInput,
                "File name does not end with .zip",
            )
        })?
        .to_string();
    // trim end matching .tar.gz or .tgz
    unroll_folder(&PathBuf::from(fname))
}

pub fn install_from_tar(path: &PathBuf) -> io::Result<()> {
    eprintln!("Installing from TAR archive: {:?}", path);
    let file = File::open(path)?;
    let mut archive = tar::Archive::new(file);
    let current_exe_path = env::current_exe().map_err(Error::other)?;
    let parent_dir = current_exe_path.parent().ok_or_else(|| {
        Error::new(
            io::ErrorKind::NotFound,
            "Current executable path has no parent",
        )
    })?;
    archive.unpack(parent_dir)?;
    let basename = path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?;
    let unrolled_path = parent_dir.join(basename);
    unroll_folder(&unrolled_path)
}

pub fn install_simple_file(path: &PathBuf) -> io::Result<()> {
    let current_exe_path = env::current_exe().map_err(Error::other)?;
    let parent_dir = current_exe_path.parent().ok_or_else(|| {
        Error::new(
            io::ErrorKind::NotFound,
            "Current executable path has no parent",
        )
    })?;
    let dest_path = parent_dir.join(path.file_name().ok_or_else(|| {
        Error::new(
            io::ErrorKind::InvalidInput,
            "Provided path has no file name",
        )
    })?);
    if dest_path.exists() {
        fs::remove_file(&dest_path).or_else(|_| fs::remove_dir_all(&dest_path))?;
    }
    fs::copy(path, dest_path)?;
    Ok(())
}

pub fn unroll_folder(path: &PathBuf) -> io::Result<()> {
    if !path.is_dir() {
        return Ok(());
    }
    let parent_dir = path.parent().ok_or_else(|| {
        Error::new(
            io::ErrorKind::NotFound,
            "Provided path has no parent directory",
        )
    })?;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let dest_path = parent_dir.join(entry.file_name());
        if dest_path.exists() {
            fs::remove_file(&dest_path).or_else(|_| fs::remove_dir_all(&dest_path))?;
        }
        fs::rename(entry.path(), dest_path)?;
    }
    fs::remove_dir_all(path)?;
    Ok(())
}

pub fn install_from_tar_gz(path: &PathBuf) -> io::Result<()> {
    eprintln!("Installing from TAR.GZ archive: {:?}", path);
    let file = File::open(path)?;
    let file = io::BufReader::new(file);
    let decompresed = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decompresed);
    // get exectuable path
    let current_exe_path = env::current_exe().map_err(Error::other)?;
    let parent_dir = current_exe_path.parent().ok_or_else(|| {
        Error::new(
            io::ErrorKind::NotFound,
            "Current executable path has no parent",
        )
    })?;

    archive.unpack(parent_dir)?;
    let fname = path
        .file_name()
        .ok_or_else(|| {
            Error::new(
                io::ErrorKind::InvalidInput,
                "Provided path has no file name",
            )
        })?
        .to_str()
        .ok_or_else(|| Error::new(io::ErrorKind::InvalidInput, "File name is not valid UTF-8"))?
        .strip_suffix(".tar.gz")
        .ok_or_else(|| {
            Error::new(
                io::ErrorKind::InvalidInput,
                "File name does not end with .tar.gz",
            )
        })?
        .to_string();
    // trim end matching .tar.gz or .tgz
    unroll_folder(&PathBuf::from(fname))
}

pub fn download_archive(url: &str) -> io::Result<PathBuf> {
    reqwest::Url::parse(url)
        .map_err(|e| Error::new(io::ErrorKind::InvalidInput, format!("Invalid URL: {e}")))?;
    let response = get(url).map_err(Error::other)?;
    // Get filename from last part of the URL
    // Try get header from Content-Disposition, if not available, use last part of the URL
    let file_name_from_content_disposition = response
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|cd| cd.to_str().ok())
        .and_then(|cd| {
            cd.split(';').find_map(|part| {
                if part.trim_start().starts_with("filename=") {
                    part.split('=')
                        .nth(1)
                        .map(|s| s.trim_matches('"').to_string())
                } else {
                    None
                }
            })
        });
    let filename_from_url = url
        .rsplit('/')
        .next()
        .ok_or_else(|| Error::new(io::ErrorKind::InvalidInput, "Invalid URL"))?
        .to_string();
    let filename = file_name_from_content_disposition.unwrap_or_else(|| filename_from_url.clone());

    let total_size = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|len| len.to_str().ok())
        .and_then(|len| len.parse::<u64>().ok());
    let pb = if unsafe { STDOUT_WRITE } {
        indicatif::ProgressBar::new(total_size.unwrap_or(0))
            .with_style(
                indicatif::ProgressStyle::default_bar()
                    .template("{spinner:.green} {msg} [{elapsed_precise}] {wide_bar} {bytes}/{total_bytes} ({eta})")
                    .unwrap(),
            )
            .with_message(format!("Downloading {filename}"))
    } else {
        indicatif::ProgressBar::hidden()
    };
    let mut source = response;
    let mut buffer = [0; 8192];
    let temp_dir = temp_dir();
    let temp_file = temp_dir.join(&filename);
    let mut dest = File::create(&temp_file).map_err(Error::other)?;
    loop {
        let n = source.read(&mut buffer).map_err(Error::other)?;
        if n == 0 {
            break; // EOF
        }
        dest.write_all(&buffer[..n]).map_err(Error::other)?;
        pb.inc(n as u64);
    }
    pb.finish_with_message("Download complete");

    Ok(temp_file)
}

pub fn download_install_archive(url: &str) -> io::Result<()> {
    let download_result = download_archive(url)?;
    p_good(
        format!(
            "Download complete, proceding to install: {}",
            download_result.display()
        )
        .as_str(),
    );
    install_archive(&download_result)
}
