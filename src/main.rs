use std::path::PathBuf;
use std::{fs, thread};

use clap::Parser;
use crossbeam_channel::Receiver;
use image::io::Reader as ImageReader;
use rayon::iter::{ParallelBridge, ParallelIterator};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Parser)]
struct Opt {
    source: PathBuf,
    destination: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let Opt { source, destination } = Opt::parse();

    let (sender, receiver) = crossbeam_channel::bounded(100);
    let _handle = thread::spawn(move || parallel_process(receiver));

    for result in WalkDir::new(&source).follow_links(true) {
        match result {
            Ok(entry) => {
                // TODO We must rewrite this part
                let destination = destination.join(entry.path());
                sender.send(Task { entry, destination })?;
            }
            Err(e) => eprintln!("{e}"),
        }
    }

    Ok(())
}

struct Task {
    entry: DirEntry,
    destination: PathBuf,
}

fn parallel_process(receiver: Receiver<Task>) {
    receiver.into_iter().par_bridge().for_each(|Task { entry, destination }| {
        let ftype = entry.file_type();
        if ftype.is_file() {
            match ImageReader::open(entry.path()) {
                Ok(reader) => match reader.decode() {
                    Ok(image) => {
                        println!("processing {:?}...", entry.path().display());
                        image.save(&destination).unwrap();
                        println!("processed {}.", destination.display());
                    }
                    Err(_) => fs::copy(entry.path(), destination).map(drop).unwrap(),
                },
                Err(e) => eprintln!("{e}"),
            }
        } else if ftype.is_dir() || ftype.is_symlink() {
            fs::create_dir_all(&destination).unwrap();
        }
    });
}
