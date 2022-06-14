use sdl2::mixer::{Channel, Chunk};

pub struct Sounds {
    wow: Chunk,
    shot: Chunk,
}

pub enum Sound {
    Wow,
    Shot,
}

impl Sounds {
    pub fn play(&self, sound: Sound) {
        let chunk = match sound {
            Sound::Wow => &self.wow,
            Sound::Shot => &self.shot,
        };

        Channel::play(Channel(-1), chunk, 0).ok();
    }
}

pub fn init_sound() -> Result<Sounds, String> {
    let _mixer_context =
        sdl2::mixer::init(sdl2::mixer::InitFlag::MP3).map_err(|e| e.to_string())?;
    // need to "open an audio device" to be able to load chunks, i.e. sound effects below
    sdl2::mixer::open_audio(
        44100,
        sdl2::mixer::DEFAULT_FORMAT,
        sdl2::mixer::DEFAULT_CHANNELS,
        1024,
    )?;

    let wow = Chunk::from_file("media/wow.mp3")?;
    let shot = Chunk::from_file("media/shot.mp3")?;

    Ok(Sounds { wow, shot })
}
