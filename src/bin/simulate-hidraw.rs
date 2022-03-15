use egalax_rs::driver::virtual_mouse;
use nix::{libc, sys::stat, unistd::mkfifo};
use std::{
    error,
    fs::{self, File, OpenOptions},
    io::{Cursor, Read, Write},
    os::unix::prelude::OpenOptionsExt,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
use tempdir::TempDir;

fn virtual_sender(data: Vec<u8>, path: PathBuf) {
    let mut writer = OpenOptions::new().write(true).open(&path).unwrap();
    let mut hidraw = Cursor::new(data);
    let mut buf = [0; 6];

    loop {
        println!("Sending next raw packet");
        let res = hidraw.read_exact(&mut buf);
        if let Ok(()) = res {
            writer.write_all(&buf).unwrap();
        } else {
            break;
        }
        thread::sleep(Duration::from_millis(500));
    }
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let hidraw = fs::read("./hidraw.bin").expect("Cannot read hidraw file");

    let tmp_dir = TempDir::new("hidraw").unwrap();
    let path = tmp_dir.path().join("egalax.fifo");
    let path1 = path.clone();
    println!("{:?}", path);
    // make a fifo to push usb data in from another thread
    mkfifo(&path, stat::Mode::S_IRWXU).unwrap();

    // a.d. the opening of both ends of the fifo is more complicated than I originally thought.
    // as explained in this answer https://stackoverflow.com/a/11637823
    // we want to read blocking, so we need to open reader as blocking.
    // therefore we need to open the writer in another thread, so that they can unblock each other.
    // we cannot open both reader and writer in the same thread, if writer is blocking we have a deadlock, if write is nonblocking, opening returns an error
    thread::spawn(move || virtual_sender(hidraw, path1));

    let reader = OpenOptions::new().read(true).open(&path).unwrap();
    println!("setup complete");

    virtual_mouse(reader)?;
    Ok(())
}
