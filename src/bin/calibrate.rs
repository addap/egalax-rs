use std::thread;
use std::time::{Duration, SystemTime};
use std::{fs::OpenOptions, io::Read};

use egalax_rs::geo::{Point, AABB};
use evdev_rs::TimeVal;
use sdl2::event::Event;
use sdl2::gfx::primitives::{DrawRenderer, ToColor};
use sdl2::keyboard::Keycode;
use sdl2::pixels;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::ttf::Font;
use sdl2::video::{Window, WindowContext};
use sdl2::Sdl;

use egalax_rs::protocol::{Packet, RawPacket, TouchState, RAW_PACKET_LEN};
use egalax_rs::units::UdimRepr;

const stage_max: usize = 4;
const pixel_coords: [(i32, i32); stage_max] = [
    (100, 100),
    (1920 - 100, 100),
    (100, 1080 - 100),
    (1920 - 100, 1080 - 100),
];

#[derive(Debug, Clone)]
struct CalibrationStage {
    stage: usize,
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
        assert!(self.stage <= stage_max);
        self.stage < stage_max
    }

    fn is_finished(&self) -> bool {
        assert!(self.stage <= stage_max);
        self.stage == stage_max
    }

    fn advance(&mut self, coord: Point) -> Result<(), String> {
        assert!(self.stage <= stage_max);

        if self.stage < stage_max {
            self.touch_coords.push(coord);
            self.stage += 1;
            Ok(())
        } else {
            Err(String::from("Already at last stage"))
        }
    }
}

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

fn render_circles(canvas: &mut Canvas<Window>, state: &CalibrationState) -> Result<(), String> {
    let red = pixels::Color::RGB(255, 0, 0);
    let green = pixels::Color::RGB(0, 255, 0);

    let colors = (0..stage_max).map(|stage| {
        if stage == state.calibration_stage.stage {
            green
        } else {
            red
        }
    });

    for (color, coords) in colors.zip(pixel_coords.iter()) {
        let x = coords.0 as i16;
        let y = coords.1 as i16;
        canvas.aa_circle(x, y, 20, color)?;
        canvas.filled_circle(x, y, 20, color)?;
    }

    Ok(())
}

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

fn render_menu(
    canvas: &mut Canvas<Window>,
    state: &CalibrationState,
    font: &Font,
) -> Result<(), String> {
    let tex_creator = canvas.texture_creator();
    let title = render_text(
        &tex_creator,
        font,
        format!("Stage {}", state.calibration_stage.stage),
    )?;
    let quit = render_text(&tex_creator, font, "(q)uit")?;
    let reset = render_text(&tex_creator, font, "(r)eset")?;
    let save = render_text(&tex_creator, font, "(s)ave")?;
    let display = render_text(
        &tex_creator,
        font,
        "Touch anywhere to visualize touch events with the current calibration.",
    )?;

    let menu = if state.calibration_stage.is_finished() {
        vec![title, quit, reset, save]
    } else {
        vec![title, quit, reset]
    };

    let (_, _, menu_area) = menu
        .iter()
        .fold((0, 0, AABB::default()), |(x, y, area), tex| {
            let q = tex.query();
            let new_area = AABB::new_wh(x, y, q.width as i32, q.height as i32);
            (x, y + (q.height as i32) + 10, area.union(new_area))
        });
    let (wwidth, wheight) = canvas.window().drawable_size();

    let (x, mut y) = (
        wwidth as i32 / 2 - menu_area.width(),
        wheight as i32 / 2 - menu_area.height() / 2,
    );
    for item in menu {
        let q = item.query();
        canvas.copy(&item, None, Some(Rect::new(x, y, q.width, q.height)))?;
        y += q.height as i32 + 10;
    }
    if state.calibration_stage.is_finished() {
        let q = display.query();
        let x = wwidth / 2 - q.width / 2;
        canvas.copy(
            &display,
            None,
            Some(Rect::new(x as i32, y, q.width, q.height)),
        )?;
        y += q.height as i32 + 10;
    }

    Ok(())
}

fn render(
    canvas: &mut Canvas<Window>,
    state: &CalibrationState,
    font: &Font,
) -> Result<(), String> {
    // clear canvas
    canvas.set_draw_color(pixels::Color::RGB(255, 255, 255));
    canvas.clear();

    render_circles(canvas, state)?;
    render_menu(canvas, state, font)?;

    canvas.present();
    Ok(())
}

fn main() -> Result<(), String> {
    env_logger::init();

    let usage = "usage: sudo ./target/debug/calibrate /dev/hidraw0";

    let node_path = std::env::args().nth(1).expect(usage);
    log::info!("Using raw device node '{}'", node_path);
    let mut device_node = OpenOptions::new().read(true).open(&node_path).unwrap();

    let sdl_context = sdl2::init()?;
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;
    let font = ttf_context.load_font("Roboto-Regular.ttf", 32)?;
    let mut canvas = init_canvas(&sdl_context)?;
    let mut events = sdl_context.event_pump()?;

    let mut state = CalibrationState::new();

    'event_loop: loop {
        // first check sdl events
        {
            let event = events.wait_event_timeout(100);

            match event {
                Some(Event::Quit { .. }) => {
                    break 'event_loop;
                }

                Some(Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                }) => match keycode {
                    Keycode::Escape | Keycode::Q => {
                        break 'event_loop;
                    }
                    Keycode::Space => {
                        if state.calibration_stage.is_finished() {
                            todo!("save calibration")
                        }
                    }
                    Keycode::R => state.reset(),
                    _ => {}
                },

                _ => {}
            }
        }

        // then try to read packets from hidraw
        {
            // thread::sleep(Duration::from_secs(1));
            let mut raw_packet: RawPacket = [0; RAW_PACKET_LEN];
            let read_bytes = device_node
                .read(&mut raw_packet)
                .map_err(|e| e.to_string())?;
            if read_bytes != RAW_PACKET_LEN {
                return Err(String::from("Did not read neough bytes"));
            }

            let packet = Packet::try_from(raw_packet).map_err(|e| e.to_string())?;

            // Either we are still in one of the four calibration stages where we collect the calibration points
            // Or we are done and will visualize the touch point based on the newly calibrated values
            state.touch_cloud.push((packet.x(), packet.y()).into());
            if state.calibration_stage.is_ongoing() {
                match (state.touch_state, packet.touch_state()) {
                    (TouchState::IsTouching, TouchState::NotTouching) => {
                        let coord = state.touch_cloud.compute_touch_coord();
                        state.touch_cloud.clear();
                        state.calibration_stage.advance(coord)?;
                    }
                    _ => {}
                }
                state.touch_state = packet.touch_state();
            }
        }

        render(&mut canvas, &state, &font)?;

        thread::sleep(Duration::from_millis(10));
    }

    Ok(())
}
