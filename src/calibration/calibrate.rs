//! Calibration program for the egalax-rs driver using SDL2

// mod audio;

// use std::collections::VecDeque;
// use std::fs::File;
// use std::sync::mpsc::{self, Receiver};
// use std::time::Duration;
// use std::{fmt, thread};
// use std::{fs::OpenOptions, io::Read};

// #[cfg(feature = "audio")]
// use crate::audio::{init_sound, Sound, Sounds};
// use egalax_rs::config::{MonitorConfig, MonitorConfigBuilder, MonitorDesignator};
// use egalax_rs::error::EgalaxError;
// use egalax_rs::geo::{Point2D, AABB};
// use egalax_rs::protocol::{MessageType, Packet, RawPacket, TouchState, RAW_PACKET_LEN};

// use egalax_rs::units::udim;
// use sdl2::event::{Event, EventSender};
// use sdl2::gfx::primitives::DrawRenderer;
// use sdl2::image::LoadTexture;
// use sdl2::keyboard::Keycode;
// use sdl2::rect::Rect;
// use sdl2::render::{Canvas, Texture, TextureCreator};
// use sdl2::ttf::Font;
// use sdl2::video::{Window, WindowContext};
// use sdl2::VideoSubsystem;
// use sdl2::{pixels, EventPump};

// /// Number of calibration points
// const STAGE_MAX: usize = 4;
// /// Number of decals recorded
// const DECALS_NUM: usize = 25;

// /// A stage in the calibration process.
// #[derive(Debug, Clone)]
// enum CalibrationStage {
//     Ongoing {
//         /// A number identifier of the stage.
//         stage: usize,
//         /// The coordinates of each individual calibration points in the coordinate system of the touch screen.
//         touch_coords: Vec<Point2D>,
//     },
//     Finished {
//         /// The final config builder that is persisted
//         saved_config: MonitorConfigBuilder,
//         /// The final config to be used.
//         decal_config: MonitorConfig,
//     },
// }

// impl Default for CalibrationStage {
//     fn default() -> Self {
//         Self::Ongoing {
//             stage: 0,
//             touch_coords: Vec::new(),
//         }
//     }
// }

// impl CalibrationStage {
//     #[allow(dead_code)]
//     fn is_ongoing(&self) -> bool {
//         match self {
//             CalibrationStage::Ongoing { .. } => true,
//             _ => false,
//         }
//     }

//     fn is_finished(&self) -> bool {
//         match self {
//             CalibrationStage::Finished { .. } => true,
//             _ => false,
//         }
//     }
// }

// impl fmt::Display for CalibrationStage {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             CalibrationStage::Ongoing { stage, .. } => {
//                 let s = format!("Stage {}", stage + 1);
//                 f.write_str(&s[..])
//             }
//             CalibrationStage::Finished { .. } => f.write_str("Finished"),
//         }
//     }
// }

// /// A collection of touch coordinates that belong to a single calibration point.
// /// The final touch coordinate of that calibration point is computed as the midpoint of the smallest area that contains the whole collection.
// struct TouchCloud {
//     v: Vec<Point2D>,
// }

// impl TouchCloud {
//     /// Compute the smallest bounding box that contains all points and then return its midpoint.
//     fn compute_touch_coord(&self) -> Point2D {
//         assert!(self.v.len() >= 1);

//         let mut abox = AABB::from(self.v[0]);

//         for point in self.v.iter().skip(1) {
//             abox = abox.grow_to_point(&point);
//         }

//         abox.midpoint()
//     }

//     fn push(&mut self, p: Point2D) {
//         self.v.push(p);
//     }

//     fn clear(&mut self) {
//         self.v.clear();
//     }
// }

// /// The state of the calibration.
// struct CalibrationState {
//     calibration_stage: CalibrationStage,
//     touch_cloud: TouchCloud,
//     touch_state: TouchState,
//     decals: VecDeque<Point2D>,
// }

// impl CalibrationState {
//     fn new() -> Self {
//         Self {
//             calibration_stage: CalibrationStage::default(),
//             touch_cloud: TouchCloud { v: Vec::new() },
//             touch_state: TouchState::NotTouching,
//             decals: VecDeque::with_capacity(DECALS_NUM),
//         }
//     }

//     fn add_decal(&mut self, decal: Point2D) {
//         if self.decals.len() == DECALS_NUM {
//             self.decals.pop_front();
//         }

//         self.decals.push_back(decal);
//     }

//     /// Add new coordinates and go to the next stage.
//     /// Switches the given calibration stage to Finished if necessary.
//     fn advance(
//         &mut self,
//         sdl_state: &SdlState,
//         coord: Point2D,
//         calibration_circle_coords: &[Point2D; STAGE_MAX],
//     ) -> Result<(), String> {
//         match &mut self.calibration_stage {
//             CalibrationStage::Ongoing {
//                 stage,
//                 touch_coords,
//             } => {
//                 touch_coords.push(coord);
//                 *stage += 1;

//                 // switch stage to finished
//                 if *stage == STAGE_MAX {
//                     if touch_coords.len() != 4 {
//                         return Err(String::from("Number of calibration points must be 4"));
//                     }

//                     // TODO the source code at https://github.com/libsdl-org/SDL/blob/main/src/video/SDL_video.c
//                     // suggests this would give us the xrandr name of the display where the program is running.
//                     // But last time we tested, the index always returned 0, and the resulting name was always the string "0".
//                     // let display_index = sdl_state.canvas.window().display_index()?;
//                     // let monitor_name = sdl_state.video_subsystem.display_name(display_index)?;

//                     // I hope these indices are all correct.
//                     let calibration_points = AABB::new(
//                         udim::average(touch_coords[0].x, touch_coords[2].x)
//                             - calibration_circle_coords[0].x,
//                         udim::average(touch_coords[0].y, touch_coords[1].y)
//                             - calibration_circle_coords[0].y,
//                         udim::average(touch_coords[3].x, touch_coords[1].x)
//                             - calibration_circle_coords[3].x,
//                         udim::average(touch_coords[3].y, touch_coords[2].y)
//                             - calibration_circle_coords[3].y,
//                     );
//                     let saved_config = MonitorConfigBuilder::new(
//                         MonitorDesignator::Named(String::from("changeme")),
//                         calibration_points,
//                     );

//                     // During the calibration we want to translate into window coordinates.
//                     // So we use the calibration points as our interpolation target. This only works if the touchscreen is the only monitor.
//                     // TODO maybe actually build the monitor and use the driver?
//                     let decal_config = MonitorConfig {
//                         screen_space: AABB::default(),
//                         monitor_area: sdl_state.monitor_area,
//                         calibration_points,
//                     };

//                     log::info!("Using config builder {:#?}", saved_config);
//                     log::info!("Using config fow showing decals {:#?}", decal_config);

//                     self.calibration_stage = CalibrationStage::Finished {
//                         saved_config,
//                         decal_config,
//                     };
//                 }

//                 Ok(())
//             }
//             CalibrationStage::Finished { .. } => Err(String::from("Already at last stage")),
//         }
//     }
// }

// struct SdlState<'ttf, 'tex> {
//     #[allow(dead_code)]
//     video_subsystem: VideoSubsystem,
//     // sdl_context: Sdl,
//     // ttf_context: &'ttf Sdl2TtfContext,
//     canvas: Canvas<Window>,
//     /// Pixel coordinates of calibration points.
//     pixel_coords: [Point2D; STAGE_MAX],
//     font: Font<'ttf, 'static>,
//     #[cfg(feature = "audio")]
//     sounds: Sounds,
//     hitmarker: Texture<'tex>,
//     monitor_area: AABB,
// }

// /// Initialize the sdl canvas and create a window.
// fn init_canvas(video_subsystem: &VideoSubsystem) -> Result<Canvas<Window>, String> {
//     let window = video_subsystem
//         .window("egalax-rs calibration", 0, 0)
//         .fullscreen_desktop()
//         .opengl()
//         .build()
//         .map_err(|e| e.to_string())?;

//     let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

//     Ok(canvas)
// }

// /// The event pump must have been polled at least calling this function.
// fn init_pixel_coords(canvas: &Canvas<Window>) -> Result<[Point2D; STAGE_MAX], String> {
//     let (wwidth, wheight) = canvas.window().drawable_size();

//     let pixel_coords: [Point2D; STAGE_MAX] = [
//         ((wwidth as f64 * 0.1) as i32, (wheight as f64 * 0.1) as i32).into(),
//         ((wwidth as f64 * 0.9) as i32, (wheight as f64 * 0.1) as i32).into(),
//         ((wwidth as f64 * 0.1) as i32, (wheight as f64 * 0.9) as i32).into(),
//         ((wwidth as f64 * 0.9) as i32, (wheight as f64 * 0.9) as i32).into(),
//     ];
//     log::info!("{:#?}", pixel_coords);

//     Ok(pixel_coords)
// }

// /// Render the calibration points as circles.
// fn render_circles(sdl_state: &SdlState, state: &CalibrationState) -> Result<(), String> {
//     let red = pixels::Color::RGB(255, 0, 0);
//     let green = pixels::Color::RGB(0, 255, 0);

//     let current_stage = if let CalibrationStage::Ongoing { stage, .. } = state.calibration_stage {
//         stage
//     } else {
//         STAGE_MAX
//     };

//     for (stage, coords) in sdl_state.pixel_coords.iter().enumerate() {
//         let color = if stage == current_stage { green } else { red };

//         let x = coords.x.value() as i16;
//         let y = coords.y.value() as i16;
//         sdl_state.canvas.aa_circle(x, y, 20, color)?;
//         sdl_state.canvas.filled_circle(x, y, 20, color)?;
//     }

//     Ok(())
// }

// /// Construct a texture out of a string of text.
// fn tex_from_text<'a>(
//     tex_creator: &'a TextureCreator<WindowContext>,
//     font: &Font,
//     text: impl AsRef<str>,
// ) -> Result<Texture<'a>, String> {
//     let surface = font
//         .render(text.as_ref())
//         .shaded(
//             pixels::Color::RGB(0, 0, 0),
//             pixels::Color::RGB(255, 255, 255),
//         )
//         .map_err(|e| e.to_string())?;
//     let tex = tex_creator
//         .create_texture_from_surface(surface)
//         .map_err(|e| e.to_string())?;
//     Ok(tex)
// }

// /// Render a hitmarker on all decal positions
// fn render_decals(sdl_state: &mut SdlState, decals: &[Point2D]) -> Result<(), String> {
//     for decal in decals {
//         let rect = Rect::new(decal.x.value() - 5, decal.y.value() - 5, 11, 11);
//         sdl_state
//             .canvas
//             .copy(&sdl_state.hitmarker, None, Some(rect))?;
//     }

//     Ok(())
// }

// /// Render the menu centered on the canvas.
// fn render_menu(sdl_state: &mut SdlState, state: &CalibrationState) -> Result<(), String> {
//     let tex_creator = sdl_state.canvas.texture_creator();
//     let title = tex_from_text(
//         &tex_creator,
//         &sdl_state.font,
//         format!("{}", state.calibration_stage),
//     )?;
//     let quit = tex_from_text(&tex_creator, &sdl_state.font, "(q)uit")?;
//     let reset = tex_from_text(&tex_creator, &sdl_state.font, "(r)eset")?;
//     let save = tex_from_text(&tex_creator, &sdl_state.font, "(s)ave")?;
//     let display = tex_from_text(
//         &tex_creator,
//         &sdl_state.font,
//         "Touch anywhere to visualize touch events with the current calibration.",
//     )?;

//     let menu = if state.calibration_stage.is_finished() {
//         vec![title, quit, reset, save, display]
//     } else {
//         vec![title, quit, reset]
//     };

//     let (wwidth, wheight) = sdl_state.canvas.window().drawable_size();

//     let mut y = wheight as i32 / 2 - 100;
//     for item in menu {
//         let q = item.query();
//         let x = wwidth / 2 - q.width / 2;
//         sdl_state
//             .canvas
//             .copy(&item, None, Some(Rect::new(x as i32, y, q.width, q.height)))?;
//         y += q.height as i32 + 10;
//     }

//     Ok(())
// }

// /// Render one frame.
// fn render(sdl_state: &mut SdlState, state: &CalibrationState) -> Result<(), String> {
//     // clear canvas
//     sdl_state
//         .canvas
//         .set_draw_color(pixels::Color::RGB(255, 255, 255));
//     sdl_state.canvas.clear();

//     render_circles(sdl_state, state)?;

//     // Don't care about order of decals so we use both slices of the VecDeque
//     // https://doc.rust-lang.org/std/collections/vec_deque/struct.VecDeque.html#method.as_slices
//     render_decals(sdl_state, state.decals.as_slices().0)?;
//     render_decals(sdl_state, state.decals.as_slices().1)?;

//     render_menu(sdl_state, state)?;

//     sdl_state.canvas.present();
//     Ok(())
// }

// /// Save the calibration state to a config file
// fn save_calibration(
//     #[cfg_attr(not(feature = "audio"), allow(unused_variables))] sdl_state: &SdlState,
//     config: &MonitorConfigBuilder,
// ) -> Result<(), EgalaxError> {
//     let f = OpenOptions::new()
//         .write(true)
//         .truncate(true)
//         .open("./config.toml")?;
//     let serialized = toml::to_string_pretty(&config)?;

//     #[cfg(feature = "audio")]
//     sdl_state.sounds.play(Sound::Wow);

//     Ok(())
// }

// fn process_sdl_events(
//     sdl_state: &SdlState,
//     state: &mut CalibrationState,
//     events: &mut EventPump,
// ) -> Result<bool, String> {
//     events.pump_events();
//     let event = events.wait_event();

//     if event.is_user_event() {
//         let packet = event
//             .as_user_event_type::<Packet>()
//             .ok_or(String::from("Unexpected custom event"))?;
//         match state.calibration_stage {
//             CalibrationStage::Ongoing { .. } => {
//                 calibrate_with_packet(sdl_state, state, packet)?;
//             }
//             CalibrationStage::Finished {
//                 decal_config: monitor_cfg,
//                 ..
//             } => {
//                 let decal = get_decal(&monitor_cfg, packet);

//                 // Noise filtering for decals
//                 if let Some(&last_decal) = state.decals.back() {
//                     if (last_decal - decal).magnitude() >= 10.0 {
//                         state.add_decal(decal)
//                     }
//                 } else {
//                     state.add_decal(decal)
//                 }
//             }
//         };
//     } else {
//         match event {
//             Event::Quit { .. } => {
//                 return Ok(true);
//             }

//             Event::KeyDown {
//                 keycode: Some(keycode),
//                 ..
//             } => match keycode {
//                 Keycode::Escape | Keycode::Q => {
//                     return Ok(true);
//                 }
//                 Keycode::S => {
//                     if let CalibrationStage::Finished {
//                         saved_config: cfg_builder,
//                         ..
//                     } = &state.calibration_stage
//                     {
//                         save_calibration(&sdl_state, cfg_builder)?;
//                     }
//                 }
//                 Keycode::R => *state = CalibrationState::new(),
//                 _ => {}
//             },
//             _ => {}
//         }
//     }

//     Ok(false)
// }

// fn calibrate_with_packet(
//     sdl_state: &SdlState,
//     state: &mut CalibrationState,
//     packet: Packet,
// ) -> Result<(), String> {
//     // If we are still in one of the four calibration stages we collect the calibration points
//     let p = (packet.x(), packet.y()).into();
//     log::info!("calibration point {:?}", p);

//     state.touch_cloud.push(p);
//     if let (TouchState::IsTouching, TouchState::NotTouching) =
//         (state.touch_state, packet.touch_state())
//     {
//         let coord = state.touch_cloud.compute_touch_coord();
//         log::info!("Set calibration point to {:?}", coord);
//         state.touch_cloud.clear();
//         state.advance(sdl_state, coord, &sdl_state.pixel_coords)?;

//         #[cfg(feature = "audio")]
//         sdl_state.sounds.play(Sound::Shot);
//     }
//     state.touch_state = packet.touch_state();

//     Ok(())
// }

// fn get_decal(monitor_cfg: &MonitorConfig, packet: Packet) -> Point2D {
//     let t = monitor_cfg.calibration_points.x().linear_factor(packet.x());
//     let x = monitor_cfg.monitor_area.x().lerp(t);

//     let t = monitor_cfg.calibration_points.y().linear_factor(packet.y());
//     let y = monitor_cfg.monitor_area.y().lerp(t);

//     let p = Point2D { x, y };

//     log::info!(
//         "Packet at ({}, {}) results in decal at {:?}",
//         packet.x(),
//         packet.y(),
//         p
//     );
//     p
// }

// fn hidraw_reader(
//     mut device_node: File,
//     sender: EventSender,
//     rx: Receiver<()>,
// ) -> Result<(), String> {
//     // try to read packets from hidraw which we either use to calibrate or to visualize the finished calibration
//     loop {
//         // Try to receive the stop signal from the main thread.
//         if let Ok(()) = rx.try_recv() {
//             return Ok(());
//         }

//         let mut raw_packet = RawPacket([0; RAW_PACKET_LEN]);
//         let read_bytes = device_node
//             .read(&mut raw_packet.0)
//             .map_err(|e| e.to_string())?;

//         if read_bytes > 0 {
//             if read_bytes != RAW_PACKET_LEN {
//                 return Err(String::from("Did not read enough bytes"));
//             }

//             let packet = Packet::try_parse(raw_packet, Some(MessageType::TouchEvent))
//                 .map_err(|e| e.to_string())?;
//             sender.push_custom_event(packet)?;
//         }
//     }
// }

// fn main() -> Result<(), String> {
//     env_logger::init();

//     let usage = "usage: sudo ./target/debug/calibrate /dev/hidraw0";

//     let node_path = std::env::args().nth(1).expect(usage);
//     log::info!("Using raw device node '{}'", node_path);
//     let device_node = OpenOptions::new().read(true).open(&node_path).unwrap();

//     let sdl_context = sdl2::init()?;
//     let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;
//     #[cfg(feature = "audio")]
//     let sounds = init_sound()?;
//     let _image_context =
//         sdl2::image::init(sdl2::image::InitFlag::JPG).map_err(|e| e.to_string())?;

//     let ev = sdl_context.event()?;
//     ev.register_custom_event::<Packet>()?;
//     // the sender is the part of the event subsystem that implements the Send trait
//     let ev_sender = ev.event_sender();
//     let (tx, rx) = mpsc::channel();
//     let hidraw_thread = thread::spawn(move || hidraw_reader(device_node, ev_sender, rx));

//     let video_subsystem = sdl_context.video()?;
//     let canvas = init_canvas(&video_subsystem)?;
//     let tex_creator = canvas.texture_creator();
//     let mut events = sdl_context.event_pump()?;

//     let (wwidth, wheight) = canvas.window().drawable_size();
//     let monitor_area = AABB::new_wh(
//         0.into(),
//         0.into(),
//         (wwidth as i32).into(),
//         (wheight as i32).into(),
//     );

//     // need to gather events once so that canvas.window().drawable_size gives the correct window size.
//     events.pump_events();
//     let pixel_coords = init_pixel_coords(&canvas)?;

//     let font = ttf_context.load_font("media/Roboto-Regular.ttf", 32)?;

//     let hitmarker = tex_creator.load_texture("media/hitmarker.png")?;

//     let mut sdl_state: SdlState = SdlState {
//         video_subsystem,
//         canvas,
//         font,
//         #[cfg(feature = "audio")]
//         sounds,
//         hitmarker,
//         pixel_coords,
//         monitor_area,
//     };

//     let mut state = CalibrationState::new();
//     render(&mut sdl_state, &state)?;

//     loop {
//         // first process sdl window/input events
//         if process_sdl_events(&sdl_state, &mut state, &mut events)? {
//             break;
//         }

//         render(&mut sdl_state, &state)?;
//         thread::sleep(Duration::from_millis(10));
//     }

//     tx.send(()).unwrap();
//     hidraw_thread.join().unwrap()
// }

fn main() {}
