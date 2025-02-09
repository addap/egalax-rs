use anyhow::anyhow;
use async_channel::TryRecvError;
use egui::{vec2, Color32, Id, Key, Painter, Pos2, Rect, Stroke, TextStyle, Vec2, ViewportId};
use evdev_rs::TimeVal;
use nix::poll::{self, PollFd, PollFlags, PollTimeout};
use std::{
    collections::VecDeque,
    fs::File,
    io::Read,
    os::fd::AsFd,
    thread::{self, JoinHandle},
};
use std::{path::Path, time::SystemTime};

#[cfg(feature = "audio")]
use super::audio::{self, Sound};
use super::{CalibratorWindowResponse, FOOTER_STYLE};
use egalax_rs::{
    config::Config,
    error::EgalaxError,
    geo::{Point2D, AABB},
    protocol::{PacketTag, RawPacket, TouchState, USBMessage, USBPacket, RAW_PACKET_LEN},
    units::{udim, Dim},
};

const PACKET_TIMEOUT_MS: u8 = 10;
const NUM_DECALS: usize = 50;
const CIRCLE_OFFSET: f32 = 0.15;
const CALIBRATION_CIRCLE_POSIIONS: [Vec2; 4] = [
    Vec2::new(CIRCLE_OFFSET, CIRCLE_OFFSET),
    Vec2::new(CIRCLE_OFFSET, 1.0 - CIRCLE_OFFSET),
    Vec2::new(1.0 - CIRCLE_OFFSET, CIRCLE_OFFSET),
    Vec2::new(1.0 - CIRCLE_OFFSET, 1.0 - CIRCLE_OFFSET),
];

#[derive(Debug, Clone)]
enum Stage {
    Stage0 { coords: [Point2D; 0] },
    Stage1 { coords: [Point2D; 1] },
    Stage2 { coords: [Point2D; 2] },
    Stage3 { coords: [Point2D; 3] },
}

impl Stage {
    fn num(&self) -> usize {
        match self {
            Self::Stage0 { .. } => 0,
            Self::Stage1 { .. } => 1,
            Self::Stage2 { .. } => 2,
            Self::Stage3 { .. } => 3,
        }
    }
}

#[derive(Debug, Clone)]
struct OngoingState {
    stage: Stage,
    touch_cloud: TouchCloud,
    touch_state: TouchState,
}

impl OngoingState {
    fn new(stage: Stage) -> Self {
        Self {
            stage,
            touch_cloud: TouchCloud { v: Vec::new() },
            touch_state: TouchState::NotTouching,
        }
    }

    /// Add new coordinates and go to the next stage.
    /// The first 3 ongoing stages just collect coordinates.
    /// If we have 4 coordinates, we use them to compute the new calibration points and switch to the Finished state.
    fn advance(&self, c: Point2D) -> CalibrationState {
        match self.stage {
            Stage::Stage0 { coords: [] } => {
                CalibrationState::Ongoing(OngoingState::new(Stage::Stage1 { coords: [c] }))
            }
            Stage::Stage1 { coords: [c0] } => {
                CalibrationState::Ongoing(OngoingState::new(Stage::Stage2 { coords: [c0, c] }))
            }
            Stage::Stage2 { coords: [c0, c1] } => {
                CalibrationState::Ongoing(OngoingState::new(Stage::Stage3 {
                    coords: [c0, c1, c],
                }))
            }
            Stage::Stage3 {
                coords: [c0, c1, c2],
            } => {
                // The four calibration points are arranged like this on the monitor
                // c0    c2
                //
                // c1    c
                //
                // We explain the following computations for the x axis, the y axis is analogous.
                // We first average the x values of c0/c1 and c2/c to find the values x0, x1
                // that we use for the calibration.
                fn average<T: Dim>(t0: udim<T>, t1: udim<T>) -> udim<T> {
                    (t0 + t1) * 0.5
                }
                let x0 = average(c0.x, c1.x);
                let x1 = average(c2.x, c.x);
                let y0 = average(c0.y, c2.y);
                let y1 = average(c1.y, c.y);

                // The goal of the calibration is to find the min and max values for the calibration bounding box.
                // The values should be such that lerp(OFFSET) = x0 & lerp(1-OFFSET) = x1.
                // where lerp(t) = (1 - t) * min + t * max.
                //
                // Unfolding the definition and solving for min & max gives us
                // min = x0 * (1 - OFFSET)/(1 - 2 * OFFSET) - x1 * OFFSET/(1 - 2 * OFFSET)
                // max = x0 + x1 - min
                let calibration_points = {
                    fn cmin<T: Dim>(t0: udim<T>, t1: udim<T>) -> udim<T> {
                        const H: f32 = 1.0 - CIRCLE_OFFSET - CIRCLE_OFFSET;
                        t0 * ((1.0 - CIRCLE_OFFSET) / H) - t1 * (CIRCLE_OFFSET / H)
                    }
                    fn cmax<T: Dim>(t0: udim<T>, t1: udim<T>, min: udim<T>) -> udim<T> {
                        t0 + t1 - min
                    }

                    let xmin = cmin(x0, x1);
                    let xmax = cmax(x0, x1, xmin);
                    let ymin = cmin(y0, y1);
                    let ymax = cmax(y0, y1, ymin);

                    AABB::new(xmin, ymin, xmax, ymax)
                };
                CalibrationState::Finished { calibration_points }
            }
        }
    }

    fn calibrate_with_packet(&mut self, packet: &USBPacket) -> Option<CalibrationState> {
        // If we are still in one of the four calibration stages we collect the calibration points
        let p = packet.position();

        self.touch_cloud.push(p);

        // When the finger is lifted, we take all the collected points, add them to that stage and go to the next.
        let result = match (self.touch_state, packet.touch_state()) {
            (TouchState::IsTouching, TouchState::NotTouching) => {
                let coord = self.touch_cloud.compute_touch_coord();
                Some(self.advance(coord))
            }
            _ => None,
        };
        self.touch_state = packet.touch_state();
        result
    }
}

/// A stage in the calibration process.
#[derive(Debug, Clone)]
enum CalibrationState {
    Ongoing(OngoingState),
    Finished {
        /// The final config to be used for further visualization.
        calibration_points: AABB,
    },
}

impl CalibrationState {
    fn start() -> Self {
        Self::Ongoing(OngoingState::new(Stage::Stage0 { coords: [] }))
    }
}

/// A collection of touch coordinates that belong to a single calibration point.
/// The final touch coordinate of that calibration point is computed as the midpoint of the smallest area that contains the whole collection.
#[derive(Debug, Clone)]
struct TouchCloud {
    v: Vec<Point2D>,
}

impl TouchCloud {
    /// Compute the smallest bounding box that contains all points and then return its midpoint.
    fn compute_touch_coord(&mut self) -> Point2D {
        assert!(!self.v.is_empty());

        let mut abox = AABB::new_wh(self.v[0].x, self.v[0].y, 0.into(), 0.into());

        for point in &self.v[1..] {
            abox = abox.grow_to_point(point);
        }

        abox.midpoint()
    }

    fn push(&mut self, p: Point2D) {
        self.v.push(p);
    }
}

pub struct Calibrator {
    rx_packet: async_channel::Receiver<USBMessage>,
    tx_exit: async_channel::Sender<()>,
    reader_handle: JoinHandle<()>,
    state: CalibrationState,
    #[cfg(feature = "audio")]
    audio_handle: audio::Handle,
    decals: VecDeque<Point2D>,
}

impl Calibrator {
    pub fn new(
        device_path: &Path,
        ctx: &egui::Context,
        #[cfg(feature = "audio")] audio_handle: audio::Handle,
    ) -> Self {
        let (tx_packet, rx_packet) = async_channel::unbounded();
        let (tx_exit, rx_exit) = async_channel::bounded(1);

        let reader_handle = thread::spawn({
            let ctx = ctx.clone();
            let device_path = device_path.to_path_buf();
            move || packet_reader(&device_path, tx_packet, rx_exit, &ctx)
        });

        Self {
            rx_packet,
            tx_exit,
            reader_handle,
            state: CalibrationState::start(),
            decals: VecDeque::with_capacity(NUM_DECALS),
            #[cfg(feature = "audio")]
            audio_handle,
        }
    }

    pub fn exit(self) {
        self.tx_exit
            .send_blocking(())
            .expect("Packet reader thread holds receiver until we send the signal.");
        self.reader_handle.join().unwrap();
    }

    pub fn update(&mut self, ctx: &egui::Context) -> CalibratorWindowResponse {
        let result = self.process_input(ctx);
        self.draw(ctx);
        result
    }

    fn process_input(&mut self, ctx: &egui::Context) -> CalibratorWindowResponse {
        // Take as many packages as are available.
        loop {
            match self.rx_packet.try_recv() {
                Ok(msg) => self.process_packet(msg.packet()),
                Err(TryRecvError::Closed) => {
                    unreachable!(
                        "The packet reader thread is active until we send an exit message."
                    )
                }
                Err(TryRecvError::Empty) => break,
            }
        }

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            return CalibratorWindowResponse::Finish(None);
        }

        match self.state {
            CalibrationState::Ongoing(_) => {}
            CalibrationState::Finished { calibration_points } => {
                if ctx.input(|i| i.key_pressed(Key::Enter)) {
                    #[cfg(feature = "audio")]
                    self.audio_handle.play(Sound::Wow);
                    return CalibratorWindowResponse::Finish(Some(calibration_points));
                }
            }
        };
        CalibratorWindowResponse::Continue
    }

    fn add_decal(&mut self, position: Point2D) {
        if self.decals.len() == NUM_DECALS {
            self.decals.pop_front();
        }
        self.decals.push_back(position);
    }

    fn process_packet(&mut self, packet: &USBPacket) {
        if let CalibrationState::Ongoing(ref mut ongoing_state) = self.state {
            if let Some(new_state) = ongoing_state.calibrate_with_packet(packet) {
                self.state = new_state;

                #[cfg(feature = "audio")]
                self.audio_handle.play(Sound::Shot);
            }
        }

        let position = packet.position();
        if let Some(last_decal) = self.decals.back() {
            if position.euclidean_distance_to(last_decal) > 10.0 {
                self.add_decal(position);
            }
        } else {
            self.add_decal(position);
        }
    }

    fn draw(&mut self, ctx: &egui::Context) {
        let srect = ctx.screen_rect();

        egui::TopBottomPanel::bottom(Id::new("footer")).show(ctx, |ui| {
            match self.state {
                CalibrationState::Ongoing(_) => {
                    let menu_items: [(&str, &str); 1] = [("Esc", "Quit & discard calibration")];
                    ui.vertical(|ui| {
                        let style = ui.style_mut();
                        style.override_text_style = Some(TextStyle::Name(FOOTER_STYLE.into()));

                        for (key, description) in menu_items {
                            let key = format!("<{key}>");
                            ui.label(format!("{key:8}- {description}"));
                        }
                    });
                }
                CalibrationState::Finished { .. } => {
                    let menu_items: [(&str, &str); 2] = [
                        ("Esc", "Quit & discard calibration"),
                        ("Enter", "Quit & accept calibration"),
                    ];

                    ui.vertical(|ui| {
                        let style = ui.style_mut();
                        style.override_text_style = Some(TextStyle::Name(FOOTER_STYLE.into()));

                        for (key, description) in menu_items {
                            let key = format!("<{key}>");
                            ui.label(format!("{key:8}- {description}"));
                        }
                    });
                }
            };
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("screen rect: {}", srect));
            ui.label(format!(
                "default calibration points: {}",
                Config::default().calibration_points
            ));
            if let CalibrationState::Finished {
                calibration_points, ..
            } = self.state
            {
                ui.label(format!("calibration points: {}", calibration_points));
            }

            let painter = ui.painter();

            // Draw the 4 calibration points. The active one is highlighted in green.
            let radius = 20.0;
            let stroke_width = 3.0;
            match self.state {
                CalibrationState::Ongoing(ref ongoing_state) => {
                    let stage_num = ongoing_state.stage.num();
                    for (i, &v) in CALIBRATION_CIRCLE_POSIIONS.iter().enumerate() {
                        let p = srect.lerp_inside(v);
                        let color = if stage_num == i {
                            Color32::GREEN
                        } else {
                            Color32::RED
                        };
                        painter.circle(p, radius, color, Stroke::new(stroke_width, Color32::BLACK));
                    }
                }
                CalibrationState::Finished { .. } => {
                    for &v in CALIBRATION_CIRCLE_POSIIONS.iter() {
                        let p = srect.lerp_inside(v);
                        let color = Color32::YELLOW;
                        painter.circle(p, radius, color, Stroke::new(stroke_width, Color32::BLACK));
                    }
                }
            }

            let painter = ui.painter();
            // Draw touch point decals as small hitmarkers.
            for p in self.decals.iter() {
                // position is in monitor coordinates so we convert to egui screen coordinates.
                let decal_default =
                    pos2_from_calibration_points(srect, Config::default().calibration_points, *p);
                self.draw_decal_default(painter, decal_default);

                if let CalibrationState::Finished {
                    calibration_points, ..
                } = self.state
                {
                    let decal_calibrated =
                        pos2_from_calibration_points(srect, calibration_points, *p);
                    self.draw_decal_calibrated(painter, decal_calibrated);
                }
            }
        });
    }

    #[allow(clippy::unused_self)]
    fn draw_decal_default(&self, painter: &Painter, decal_pos: Pos2) {
        let d: f32 = 5.0;
        let decal_stroke = Stroke::new(1.0, Color32::BLACK);
        painter.line_segment(
            [decal_pos + vec2(-d, -d), decal_pos + vec2(d, d)],
            decal_stroke,
        );
        painter.line_segment(
            [decal_pos + vec2(-d, d), decal_pos + vec2(d, -d)],
            decal_stroke,
        );
    }

    #[allow(clippy::unused_self)]
    fn draw_decal_calibrated(&self, painter: &Painter, decal_pos: Pos2) {
        let d: f32 = 5.0;
        let decal_stroke = Stroke::new(1.0, Color32::RED);
        painter.line_segment(
            [decal_pos + vec2(-d, 0.0), decal_pos + vec2(d, 0.0)],
            decal_stroke,
        );
        painter.line_segment(
            [decal_pos + vec2(0.0, d), decal_pos + vec2(0.0, -d)],
            decal_stroke,
        );
    }
}

fn pos2_from_calibration_points(srect: Rect, calibration_points: AABB, position: Point2D) -> Pos2 {
    let x_scale = calibration_points.xrange().linear_factor(position.x);
    let y_scale = calibration_points.yrange().linear_factor(position.y);

    srect.lerp_inside(vec2(x_scale, y_scale))
}

/// Infinite loop of reading from the device node with a timeout and checking the `rx_exit` receiver for a stop signal.
fn packet_reader(
    device_path: &Path,
    tx_packet: async_channel::Sender<USBMessage>,
    rx_exit: async_channel::Receiver<()>,
    ctx: &egui::Context,
) {
    let device_node = File::open(device_path).unwrap_or_else(|_| {
        panic!(
            "Opening `{:?}` failed. USB cable to monitor disconnected?",
            device_path
        )
    });
    log::info!("Opened device node `{:?}`", device_path);

    fn read_packets(
        mut device_node: File,
        tx_packet: async_channel::Sender<USBMessage>,
        rx_exit: async_channel::Receiver<()>,
        ctx: &egui::Context,
    ) -> Result<(), EgalaxError> {
        log::trace!("Entering read_packets.");

        let mut raw_packet = RawPacket([0; RAW_PACKET_LEN]);

        'main: loop {
            match rx_exit.try_recv() {
                Ok(()) => {
                    log::info!("Packet reader received exit signal");
                    break 'main;
                }
                // No exit signal, so try to read from device.
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Closed) => unreachable!(
                    "The sender is only dropped after the packet reader thread has finished"
                ),
            }

            let mut fds = [PollFd::new(device_node.as_fd(), PollFlags::POLLIN)];
            match poll::poll(&mut fds, PollTimeout::from(PACKET_TIMEOUT_MS)) {
                // Timeout, nothing to check
                Ok(0) => continue 'main,
                // Device node is ready.
                Ok(1) => {
                    match device_node.read(&mut raw_packet.0) {
                        Ok(RAW_PACKET_LEN) => {
                            log::info!("Read raw packet: {}", raw_packet);

                            let time = TimeVal::try_from(SystemTime::now())?;
                            let packet =
                                USBPacket::try_parse(raw_packet, Some(PacketTag::TouchEvent))?;
                            let msg = packet.with_time(time);

                            match tx_packet.send_blocking(msg) {
                                Err(_) => {
                                    return Err(anyhow!(
                                        "The packet receiver unexpectedly hung up."
                                    )
                                    .into())
                                }
                                Ok(()) => ctx.request_repaint_of(ViewportId(Id::new("calibrator"))),
                            };
                        }
                        // I think if the monitor sends a packet, then the kernel will make all bytes available at once.
                        // Therefore, in a situation where I cannot read RAW_PACKET_LEN bytes I want to fail fast.
                        Ok(_) => {
                            return Err(anyhow!("Read partial packet.").into());
                        }
                        Err(e) => {
                            return Err(e.into());
                        }
                    };
                }
                Err(errno) => {
                    return Err(anyhow!("Error during poll: {errno}.").into());
                }
                _ => unreachable!("Only possible return values are 0, 1 or error."),
            }
        }

        log::trace!("Leaving read_packets.");
        Ok(())
    }

    if let Err(e) = read_packets(device_node, tx_packet, rx_exit, ctx) {
        eprintln!("Calibrator packet reader thread encountered an error:\n{e}");
    }
}
