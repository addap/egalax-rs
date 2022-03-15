use nix::sys::stat;
use nix::unistd::mkfifo;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::path::PathBuf;
use std::str;
use std::thread;
use std::time;
use tempdir::TempDir;

fn sender(path: PathBuf) {
    let mut f = OpenOptions::new().write(true).open(path).unwrap();
    loop {
        f.write("My Data".as_bytes()).unwrap();
        thread::sleep(time::Duration::from_secs(1));
    }
}

fn receiver(path: PathBuf) {
    let mut f = File::open(path).unwrap();
    let mut buf = [0; 10];
    loop {
        f.read(&mut buf[..]).unwrap();
        println!("{}", str::from_utf8(&buf).unwrap());
        thread::sleep(time::Duration::from_secs(2));
    }
}

fn main() {
    let tmp_dir = TempDir::new("test_fifo").unwrap();
    let path = tmp_dir.path().join("myfifo");
    let path1 = path.clone();
    let path2 = path.clone();
    mkfifo(&path, stat::Mode::S_IRWXU).unwrap();

    thread::spawn(|| sender(path1));
    thread::spawn(|| receiver(path2));

    loop {
        thread::sleep(time::Duration::from_secs(10));
    }
}
