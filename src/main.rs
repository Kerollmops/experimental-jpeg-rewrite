use std::path::PathBuf;
use std::{fs, thread};

use anyhow::Context;
use clap::Parser;
use crossbeam_channel::Receiver;
use image::io::Reader as ImageReader;
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use rayon::iter::{ParallelBridge, ParallelIterator};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Parser)]
struct Opt {
    source: PathBuf,
    destination: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let Opt { source, destination } = Opt::parse();

    let total_count = WalkDir::new(&source).follow_links(true).into_iter().count() as u64;
    let style = ProgressStyle::with_template("{wide_bar} {human_pos}/{human_len} {eta}").unwrap();
    let bar = ProgressBar::new(total_count).with_style(style);

    let (sender, receiver) = crossbeam_channel::bounded(100);
    let handle = thread::spawn(move || parallel_process(bar, receiver));

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

    drop(sender);
    handle.join().unwrap()?;

    Ok(())
}

struct Task {
    entry: DirEntry,
    destination: PathBuf,
}

fn parallel_process(bar: ProgressBar, receiver: Receiver<Task>) -> anyhow::Result<()> {
    receiver.into_iter().par_bridge().progress_with(bar).try_for_each(
        |Task { entry, destination }| {
            let ftype = entry.file_type();
            if ftype.is_file() {
                match ImageReader::open(entry.path()) {
                    Ok(reader) => match reader.decode() {
                        Ok(image) => image.save(&destination)?,
                        Err(_) => {
                            fs::copy(entry.path(), &destination).map(drop).with_context(|| {
                                format!(
                                    "Copying {} into {}",
                                    entry.path().display(),
                                    destination.display()
                                )
                            })?
                        }
                    },
                    Err(e) => eprintln!("{e}"),
                }
            } else if ftype.is_dir() || ftype.is_symlink() {
                fs::create_dir_all(&destination)?;
            }
            Ok(())
        },
    )
}
