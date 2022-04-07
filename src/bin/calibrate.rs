use std::time::Duration;
use std::{fmt, thread};
use std::{fs::OpenOptions, io::Read};

use egalax_rs::config::MonitorConfigBuilder;
use egalax_rs::geo::{Point, AABB};
use sdl2::event::Event;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::keyboard::Keycode;
use sdl2::mixer::{Channel, Chunk};
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::ttf::Font;
use sdl2::video::{Window, WindowContext};
use sdl2::Sdl;
use sdl2::{pixels, EventPump};

use egalax_rs::protocol::{Packet, RawPacket, TouchState, RAW_PACKET_LEN};

/// Number of calibration points
const STAGE_MAX: usize = 4;

/// Pixel coordinates of calibration points.
/// TODO should be computed from canvas.window().drawable_area
const PIXEL_COORDS: [(i32, i32); STAGE_MAX] = [
    (100, 100),
    (1920 - 100, 100),
    (100, 1080 - 100),
    (1920 - 100, 1080 - 100),
];

/// A stage in the calibration process.
#[derive(Debug, Clone)]
struct CalibrationStage {
    /// A number identifier of the stage.
    stage: usize,
    /// The coordinates of each individual calibration points in the coordinate system of the touch screen.
    touch_coords: Vec<Point>,
}

impl CalibrationStage {
    fn new() -> Self {
        Self {
            stage: 0,
            touch_coords: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.stage = 0;
        self.touch_coords.clear();
    }

    fn is_ongoing(&self) -> bool {
        assert!(self.stage <= STAGE_MAX);
        self.stage < STAGE_MAX
    }

    fn is_finished(&self) -> bool {
        assert!(self.stage <= STAGE_MAX);
        self.stage == STAGE_MAX
    }

    /// Add new coordinates and go to the next stage.
    fn advance(&mut self, coord: Point) -> Result<(), String> {
        assert!(self.stage <= STAGE_MAX);

        if self.stage < STAGE_MAX {
            self.touch_coords.push(coord);
            self.stage += 1;
            Ok(())
        } else {
            Err(String::from("Already at last stage"))
        }
    }
}

impl fmt::Display for CalibrationStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_finished() {
            f.write_str("Finished")
        } else {
            let s = format!("Stage {}", self.stage + 1);
            f.write_str(&s[..])
        }
    }
}

/// A collection of touch coordinates that belong to a single calibration point.
/// The final touch coordinate of that calibration point is computed as the midpoint of the smallest area that contains the whole collection.
struct TouchCloud {
    v: Vec<Point>,
}

impl TouchCloud {
    fn compute_touch_coord(&self) -> Point {
        let mut abox = AABB::default();

        for point in &self.v {
            abox = abox.grow_to_point(&point);
        }

        abox.midpoint()
    }

    fn push(&mut self, p: Point) {
        self.v.push(p);
    }

    fn clear(&mut self) {
        self.v.clear();
    }
}

/// The state of the calibration.
struct CalibrationState {
    calibration_stage: CalibrationStage,
    touch_cloud: TouchCloud,
    touch_state: TouchState,
}

impl CalibrationState {
    fn new() -> Self {
        Self {
            calibration_stage: CalibrationStage::new(),
            touch_cloud: TouchCloud { v: Vec::new() },
            touch_state: TouchState::NotTouching,
        }
    }

    fn reset(&mut self) {
        self.calibration_stage.reset();
        self.touch_cloud.clear();
        self.touch_state = TouchState::NotTouching;
    }
}

struct SdlState<'ttf> {
    // sdl_context: Sdl,
    // ttf_context: &'ttf Sdl2TtfContext,
    canvas: Canvas<Window>,
    font: Font<'ttf, 'static>,
    wow: Chunk,
    shot: Chunk,
}

/// Initialize the sdl canvas and create a window.
fn init_canvas(sdl_context: &Sdl) -> Result<Canvas<Window>, String> {
    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window("egalax-rs calibration", 0, 0)
        .fullscreen_desktop()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    Ok(canvas)
}

/// Render the calibration points as circles.
fn render_circles(sdl_state: &SdlState, state: &CalibrationState) -> Result<(), String> {
    let red = pixels::Color::RGB(255, 0, 0);
    let green = pixels::Color::RGB(0, 255, 0);

    let colors = (0..STAGE_MAX).map(|stage| {
        if stage == state.calibration_stage.stage {
            green
        } else {
            red
        }
    });

    for (color, coords) in colors.zip(PIXEL_COORDS.iter()) {
        let x = coords.0 as i16;
        let y = coords.1 as i16;
        sdl_state.canvas.aa_circle(x, y, 20, color)?;
        sdl_state.canvas.filled_circle(x, y, 20, color)?;
    }

    Ok(())
}

/// Construct a texture out of a string of text.
fn render_text<'a>(
    tex_creator: &'a TextureCreator<WindowContext>,
    font: &Font,
    text: impl AsRef<str>,
) -> Result<Texture<'a>, String> {
    let surface = font
        .render(text.as_ref())
        .shaded(
            pixels::Color::RGB(0, 0, 0),
            pixels::Color::RGB(255, 255, 255),
        )
        .map_err(|e| e.to_string())?;
    let tex = tex_creator
        .create_texture_from_surface(surface)
        .map_err(|e| e.to_string())?;
    Ok(tex)
}

/// Render the menu centered on the canvas.
fn render_menu(sdl_state: &mut SdlState, state: &CalibrationState) -> Result<(), String> {
    let tex_creator = sdl_state.canvas.texture_creator();
    let title = render_text(
        &tex_creator,
        &sdl_state.font,
        format!("{}", state.calibration_stage),
    )?;
    let quit = render_text(&tex_creator, &sdl_state.font, "(q)uit")?;
    let reset = render_text(&tex_creator, &sdl_state.font, "(r)eset")?;
    let save = render_text(&tex_creator, &sdl_state.font, "(s)ave")?;
    let display = render_text(
        &tex_creator,
        &sdl_state.font,
        "Touch anywhere to visualize touch events with the current calibration.",
    )?;

    let menu = if state.calibration_stage.is_finished() {
        vec![title, quit, reset, save, display]
    } else {
        vec![title, quit, reset]
    };

    let (wwidth, wheight) = sdl_state.canvas.window().drawable_size();

    let mut y = wheight as i32 / 2 - 100;
    for item in menu {
        let q = item.query();
        let x = wwidth / 2 - q.width / 2;
        sdl_state
            .canvas
            .copy(&item, None, Some(Rect::new(x as i32, y, q.width, q.height)))?;
        y += q.height as i32 + 10;
    }

    Ok(())
}

/// Render one frame.
fn render(sdl_state: &mut SdlState, state: &CalibrationState) -> Result<(), String> {
    // clear canvas
    sdl_state
        .canvas
        .set_draw_color(pixels::Color::RGB(255, 255, 255));
    sdl_state.canvas.clear();

    render_circles(sdl_state, state)?;
    render_menu(sdl_state, state)?;

    sdl_state.canvas.present();
    Ok(())
}

/// Save the calibration state to a config file
fn save_calibration(sdl_state: &SdlState, state: &CalibrationState) -> Result<(), String> {
    if state.calibration_stage.touch_coords.len() != 4 {
        return Err(String::from("Number of calibration points must be 4"));
    }

    // TODO don't just take entries 0 and 3. should we average them with entries 1 & 2?
    let calibration_points = AABB::new(
        state.calibration_stage.touch_coords[0].x.value(),
        state.calibration_stage.touch_coords[0].y.value(),
        state.calibration_stage.touch_coords[3].x.value(),
        state.calibration_stage.touch_coords[3].y.value(),
    );
    let calibration_margins_px = AABB::new(
        PIXEL_COORDS[0].0,
        PIXEL_COORDS[0].1,
        PIXEL_COORDS[3].0,
        PIXEL_COORDS[3].1,
    );
    let config = MonitorConfigBuilder::new(None, calibration_points, calibration_margins_px);

    let f = OpenOptions::new()
        .write(true)
        .open("./config")
        .map_err(|e| e.to_string())?;
    serde_lexpr::to_writer(f, &config).map_err(|e| e.to_string())?;

    Channel::play(Channel(-1), &sdl_state.wow, 0)?;

    Ok(())
}

fn process_sdl_events(
    sdl_state: &SdlState,
    state: &mut CalibrationState,
    events: &mut EventPump,
) -> Result<bool, String> {
    let event = events.wait_event_timeout(100);

    match event {
        Some(Event::Quit { .. }) => {
            return Ok(true);
        }

        Some(Event::KeyDown {
            keycode: Some(keycode),
            ..
        }) => match keycode {
            Keycode::Escape | Keycode::Q => {
                return Ok(true);
            }
            Keycode::S => {
                if state.calibration_stage.is_finished() {
                    save_calibration(&sdl_state, &state)?;
                }
            }
            Keycode::R => state.reset(),
            _ => {}
        },

        _ => {}
    }

    Ok(false)
}

fn process_usb_packets(
    sdl_state: &SdlState,
    state: &mut CalibrationState,
    mut stream: impl Read,
) -> Result<(), String> {
    let mut raw_packet: RawPacket = [0; RAW_PACKET_LEN];
    let read_bytes = stream.read(&mut raw_packet).map_err(|e| e.to_string())?;

    if read_bytes > 0 {
        if read_bytes != RAW_PACKET_LEN {
            return Err(String::from("Did not read neough bytes"));
        }

        let packet = Packet::try_from(raw_packet).map_err(|e| e.to_string())?;

        // If we are still in one of the four calibration stages we collect the calibration points
        state.touch_cloud.push((packet.x(), packet.y()).into());
        if state.calibration_stage.is_ongoing() {
            match (state.touch_state, packet.touch_state()) {
                (TouchState::IsTouching, TouchState::NotTouching) => {
                    let coord = state.touch_cloud.compute_touch_coord();
                    state.touch_cloud.clear();
                    state.calibration_stage.advance(coord)?;

                    Channel::play(Channel(-1), &sdl_state.shot, 0)?;
                }
                _ => {}
            }
            state.touch_state = packet.touch_state();
        }
    }

    Ok(())
}

fn main() -> Result<(), String> {
    env_logger::init();

    let usage = "usage: sudo ./target/debug/calibrate /dev/hidraw0";

    let node_path = std::env::args().nth(1).expect(usage);
    log::info!("Using raw device node '{}'", node_path);
    let device_node = OpenOptions::new().read(true).open(&node_path).unwrap();

    let sdl_context = sdl2::init()?;
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;
    let _mixer_context =
        sdl2::mixer::init(sdl2::mixer::InitFlag::MP3).map_err(|e| e.to_string())?;
    // need to "open an audio device" to be able to load chunks, i.e. sound effects below
    sdl2::mixer::open_audio(
        44100,
        sdl2::mixer::DEFAULT_FORMAT,
        sdl2::mixer::DEFAULT_CHANNELS,
        1024,
    )?;

    let canvas = init_canvas(&sdl_context)?;
    let mut events = sdl_context.event_pump()?;

    let font = ttf_context.load_font("Roboto-Regular.ttf", 32)?;

    let wow = Chunk::from_file("media/wow.mp3")?;
    let shot = Chunk::from_file("media/shot.mp3")?;

    let mut sdl_state: SdlState = SdlState {
        canvas,
        font,
        wow,
        shot,
    };
    let mut state = CalibrationState::new();

    'event_loop: loop {
        // first process sdl window/input events
        if process_sdl_events(&sdl_state, &mut state, &mut events)? {
            break 'event_loop;
        }

        // then try to read packets from hidraw
        process_usb_packets(&sdl_state, &mut state, &device_node)?;
        render(&mut sdl_state, &state)?;
        thread::sleep(Duration::from_millis(10));
    }

    Ok(())
}
