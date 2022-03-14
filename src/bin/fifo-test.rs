extern crate nix;

use nix::sys::stat;
use nix::unistd::mkfifo;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::str;
use std::thread;
use std::time;

fn sender(path: &str) {
    let mut f = OpenOptions::new().write(true).open(path).unwrap();
    loop {
        f.write("My Data".as_bytes()).unwrap();
        thread::sleep(time::Duration::from_secs(1));
    }
}

fn receiver(path: &str) {
    let mut f = File::open(path).unwrap();
    let mut buf = [0; 10];
    loop {
        f.read(&mut buf[..]).unwrap();
        println!("{}", str::from_utf8(&buf).unwrap());
        thread::sleep(time::Duration::from_secs(2));
    }
}

fn main() {
    let path = "/tmp/myfifo";
    mkfifo(path, stat::Mode::S_IRWXU).unwrap();

    thread::spawn(|| sender(path));
    thread::spawn(|| receiver(path));

    loop {
        thread::sleep(time::Duration::from_secs(10));
    }
}
