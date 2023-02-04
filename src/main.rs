use std::fs;
use std::path::PathBuf;

use clap::Parser;
use image::io::Reader as ImageReader;
use walkdir::WalkDir;

#[derive(Debug, Parser)]
struct Opt {
    source: PathBuf,
    destination: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let Opt { source, destination } = Opt::parse();

    for result in WalkDir::new(&source).follow_links(true) {
        match result {
            Ok(entry) => {
                let ftype = entry.file_type();
                // TODO We must rewrite this part
                let destination = destination.join(entry.path());
                if ftype.is_file() {
                    match ImageReader::open(entry.path()) {
                        Ok(reader) => match reader.decode() {
                            Ok(image) => {
                                println!("processing {:?}...", entry.path().display());
                                image.save(&destination)?;
                                println!("processed {}.", destination.display());
                            }
                            Err(_) => fs::copy(entry.path(), destination).map(drop)?,
                        },
                        Err(e) => eprintln!("{e}"),
                    }
                } else if ftype.is_dir() || ftype.is_symlink() {
                    fs::create_dir_all(&destination)?;
                }
            }
            Err(e) => eprintln!("{e}"),
        }
    }

    Ok(())
}
