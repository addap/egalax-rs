use rodio::{source::Source, Decoder, OutputStream, OutputStreamHandle};
use std::io;

const WOW: &'static [u8] = include_bytes!("../../media/wow.mp3");
const SHOT: &'static [u8] = include_bytes!("../../media/shot.mp3");

pub struct SoundManager {
    _stream: OutputStream,
    handle: Handle,
}

#[derive(Clone)]
pub struct Handle {
    stream_handle: OutputStreamHandle,
    shot: SoundData,
    wow: SoundData,
}

pub enum Sound {
    Wow,
    Shot,
}

/// Based on solutions in https://github.com/RustAudio/rodio/issues/141
#[derive(Debug, Clone)]
pub struct SoundData(&'static [u8]);

impl SoundData {
    fn new(buf: &'static [u8]) -> Self {
        Self(buf)
    }
    fn decoder(self: &Self) -> Decoder<io::Cursor<&'static [u8]>> {
        let cursor = io::Cursor::new(self.0);
        rodio::Decoder::new(cursor).unwrap()
    }
}

impl SoundManager {
    pub fn init() -> Result<Self, anyhow::Error> {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let wow = SoundData::new(WOW);
        let shot = SoundData::new(SHOT);
        let handle = Handle {
            stream_handle,
            shot,
            wow,
        };

        Ok(Self { _stream, handle })
    }

    pub fn get_handle(&self) -> Handle {
        self.handle.clone()
    }
}

impl Handle {
    pub fn play(&self, sound: Sound) {
        let sound_data = match sound {
            Sound::Wow => self.wow.decoder(),
            Sound::Shot => self.shot.decoder(),
        };

        self.stream_handle
            .play_raw(sound_data.convert_samples())
            .ok();
    }
}
