#[cfg(feature = "audio")]
mod audio;
mod calibrate;

use const_format::formatcp;
use egui::{vec2, Color32, FontId, Id, Key, TextStyle, Theme, ViewportBuilder, ViewportClass};
use evdev_rs::enums::EV_KEY;
use std::{mem, path::PathBuf};

use calibrate::Calibrator;
use egalax_rs::{
    config::{self, ConfigFile},
    geo::AABB,
};

const CONFIG_FILE_PATH: &str = "./config.toml";
const FOOTER_STYLE: &str = "footer";
const CONTENT_OFFSET: f32 = 0.3;
const EV_KEYS: [EV_KEY; 108] = [
    EV_KEY::BTN_0,
    EV_KEY::BTN_1,
    EV_KEY::BTN_2,
    EV_KEY::BTN_3,
    EV_KEY::BTN_4,
    EV_KEY::BTN_5,
    EV_KEY::BTN_6,
    EV_KEY::BTN_7,
    EV_KEY::BTN_8,
    EV_KEY::BTN_9,
    EV_KEY::BTN_LEFT,
    EV_KEY::BTN_RIGHT,
    EV_KEY::BTN_MIDDLE,
    EV_KEY::BTN_SIDE,
    EV_KEY::BTN_EXTRA,
    EV_KEY::BTN_FORWARD,
    EV_KEY::BTN_BACK,
    EV_KEY::BTN_TASK,
    EV_KEY::BTN_TRIGGER,
    EV_KEY::BTN_THUMB,
    EV_KEY::BTN_THUMB2,
    EV_KEY::BTN_TOP,
    EV_KEY::BTN_TOP2,
    EV_KEY::BTN_PINKIE,
    EV_KEY::BTN_BASE,
    EV_KEY::BTN_BASE2,
    EV_KEY::BTN_BASE3,
    EV_KEY::BTN_BASE4,
    EV_KEY::BTN_BASE5,
    EV_KEY::BTN_BASE6,
    EV_KEY::BTN_DEAD,
    EV_KEY::BTN_SOUTH,
    EV_KEY::BTN_EAST,
    EV_KEY::BTN_C,
    EV_KEY::BTN_NORTH,
    EV_KEY::BTN_WEST,
    EV_KEY::BTN_Z,
    EV_KEY::BTN_TL,
    EV_KEY::BTN_TR,
    EV_KEY::BTN_TL2,
    EV_KEY::BTN_TR2,
    EV_KEY::BTN_SELECT,
    EV_KEY::BTN_START,
    EV_KEY::BTN_MODE,
    EV_KEY::BTN_THUMBL,
    EV_KEY::BTN_THUMBR,
    EV_KEY::BTN_TOOL_PEN,
    EV_KEY::BTN_TOOL_RUBBER,
    EV_KEY::BTN_TOOL_BRUSH,
    EV_KEY::BTN_TOOL_PENCIL,
    EV_KEY::BTN_TOOL_AIRBRUSH,
    EV_KEY::BTN_TOOL_FINGER,
    EV_KEY::BTN_TOOL_MOUSE,
    EV_KEY::BTN_TOOL_LENS,
    EV_KEY::BTN_TOOL_QUINTTAP,
    EV_KEY::BTN_STYLUS3,
    EV_KEY::BTN_TOUCH,
    EV_KEY::BTN_STYLUS,
    EV_KEY::BTN_STYLUS2,
    EV_KEY::BTN_TOOL_DOUBLETAP,
    EV_KEY::BTN_TOOL_TRIPLETAP,
    EV_KEY::BTN_TOOL_QUADTAP,
    EV_KEY::BTN_GEAR_DOWN,
    EV_KEY::BTN_GEAR_UP,
    EV_KEY::BTN_DPAD_UP,
    EV_KEY::BTN_DPAD_DOWN,
    EV_KEY::BTN_DPAD_LEFT,
    EV_KEY::BTN_DPAD_RIGHT,
    EV_KEY::BTN_TRIGGER_HAPPY1,
    EV_KEY::BTN_TRIGGER_HAPPY2,
    EV_KEY::BTN_TRIGGER_HAPPY3,
    EV_KEY::BTN_TRIGGER_HAPPY4,
    EV_KEY::BTN_TRIGGER_HAPPY5,
    EV_KEY::BTN_TRIGGER_HAPPY6,
    EV_KEY::BTN_TRIGGER_HAPPY7,
    EV_KEY::BTN_TRIGGER_HAPPY8,
    EV_KEY::BTN_TRIGGER_HAPPY9,
    EV_KEY::BTN_TRIGGER_HAPPY10,
    EV_KEY::BTN_TRIGGER_HAPPY11,
    EV_KEY::BTN_TRIGGER_HAPPY12,
    EV_KEY::BTN_TRIGGER_HAPPY13,
    EV_KEY::BTN_TRIGGER_HAPPY14,
    EV_KEY::BTN_TRIGGER_HAPPY15,
    EV_KEY::BTN_TRIGGER_HAPPY16,
    EV_KEY::BTN_TRIGGER_HAPPY17,
    EV_KEY::BTN_TRIGGER_HAPPY18,
    EV_KEY::BTN_TRIGGER_HAPPY19,
    EV_KEY::BTN_TRIGGER_HAPPY20,
    EV_KEY::BTN_TRIGGER_HAPPY21,
    EV_KEY::BTN_TRIGGER_HAPPY22,
    EV_KEY::BTN_TRIGGER_HAPPY23,
    EV_KEY::BTN_TRIGGER_HAPPY24,
    EV_KEY::BTN_TRIGGER_HAPPY25,
    EV_KEY::BTN_TRIGGER_HAPPY26,
    EV_KEY::BTN_TRIGGER_HAPPY27,
    EV_KEY::BTN_TRIGGER_HAPPY28,
    EV_KEY::BTN_TRIGGER_HAPPY29,
    EV_KEY::BTN_TRIGGER_HAPPY30,
    EV_KEY::BTN_TRIGGER_HAPPY31,
    EV_KEY::BTN_TRIGGER_HAPPY32,
    EV_KEY::BTN_TRIGGER_HAPPY33,
    EV_KEY::BTN_TRIGGER_HAPPY34,
    EV_KEY::BTN_TRIGGER_HAPPY35,
    EV_KEY::BTN_TRIGGER_HAPPY36,
    EV_KEY::BTN_TRIGGER_HAPPY37,
    EV_KEY::BTN_TRIGGER_HAPPY38,
    EV_KEY::BTN_TRIGGER_HAPPY39,
    EV_KEY::BTN_TRIGGER_HAPPY40,
];

struct Input {
    has_moved: String,
    right_click_wait: String,
}

impl Input {
    fn new(config_file: &ConfigFile) -> Self {
        Self {
            has_moved: config_file.common.has_moved_threshold.to_string(),
            right_click_wait: config_file.common.right_click_wait_ms.to_string(),
        }
    }
}

enum CalibratorWindowState {
    Deactivated,
    Running(Calibrator),
}

enum CalibratorWindowResponse {
    Continue,
    Finish(Option<AABB>),
}

pub struct App {
    current_config: ConfigFile,
    original_config: ConfigFile,
    input: Input,
    monitors: Vec<String>,
    device_path: PathBuf,
    calibrator_window: CalibratorWindowState,
    #[cfg(feature = "audio")]
    sound_manager: audio::SoundManager,
}

impl App {
    pub fn new(
        device_path: PathBuf,
        monitors: Vec<String>,
        cc: &eframe::CreationContext<'_>,
    ) -> Self {
        cc.egui_ctx
            .options_mut(|options| options.fallback_theme = Theme::Light);

        let config_file = ConfigFile::from_file(CONFIG_FILE_PATH).unwrap_or_default();
        let input = Input::new(&config_file);

        #[cfg(feature = "audio")]
        let sound_manager = audio::SoundManager::init().unwrap();

        Self {
            current_config: config_file.clone(),
            original_config: config_file,
            input,
            monitors,
            device_path,
            calibrator_window: CalibratorWindowState::Deactivated,
            #[cfg(feature = "audio")]
            sound_manager,
        }
    }

    /// Handle key events in the main view.
    /// Esc   - close without saving
    /// Enter - close and save config file
    /// c     - start calibration
    fn process_input(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if ctx.input(|i| i.key_pressed(Key::Enter)) {
            if let Err(e) = self.current_config.save_file(CONFIG_FILE_PATH) {
                eprintln!("{}", e);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if ctx.input(|i| i.key_pressed(Key::C)) {
            self.start_calibration(ctx);
        } else if ctx.input(|i| i.key_pressed(Key::R)) {
            self.current_config = self.original_config.clone();
        }
    }

    fn draw(&mut self, ctx: &egui::Context) {
        let srect = ctx.screen_rect();
        ctx.style_mut(|style| {
            style.text_styles.insert(
                egui::TextStyle::Heading,
                FontId::new(36.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                TextStyle::Name(FOOTER_STYLE.into()),
                FontId::new(28.0, egui::FontFamily::Monospace),
            );
        });

        egui::TopBottomPanel::top(Id::new("header")).show(ctx, |ui| {
            ui.heading("egalax-rs Settings Editor");
        });
        egui::TopBottomPanel::bottom(Id::new("footer")).show(ctx, |ui| {
            let menu_items: [(&str, &str); 4] = [
                ("Esc", "Quit"),
                ("r", "Reset"),
                ("c", "Start Calibrator"),
                (
                    "Enter",
                    formatcp!("Quit & save to \"{}\"", CONFIG_FILE_PATH),
                ),
            ];

            ui.vertical(|ui| {
                let style = ui.style_mut();
                style.override_text_style = Some(TextStyle::Name(FOOTER_STYLE.into()));

                for (key, description) in menu_items {
                    let key = format!("<{key}>");
                    ui.label(format!("{key:8}- {description}"));
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.add_space(srect.lerp_inside(vec2(0.0, CONTENT_OFFSET)).y);
                ui.horizontal(|ui| {
                    ui.label("Monitors: ");
                    egui::ComboBox::from_id_salt(0)
                        .selected_text(self.current_config.monitor_designator.to_string())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.current_config.monitor_designator,
                                config::MonitorDesignator::Primary,
                                config::MonitorDesignator::Primary.to_string(),
                            );
                            for monitor in self.monitors.iter() {
                                ui.selectable_value(
                                    &mut self.current_config.monitor_designator,
                                    config::MonitorDesignator::Named(monitor.clone()),
                                    monitor,
                                );
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Left-Click Event ({:?}): ",
                        self.current_config.common.ev_left_click
                    ));
                    egui::ComboBox::from_id_salt(1)
                        .selected_text(format!("{:?}", self.current_config.common.ev_left_click))
                        .show_ui(ui, |ui| {
                            for ev_key in EV_KEYS {
                                ui.selectable_value(
                                    &mut self.current_config.common.ev_left_click,
                                    ev_key,
                                    format!("{:?}", ev_key),
                                );
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Right-Click Event ({:?}): ",
                        self.current_config.common.ev_right_click
                    ));
                    egui::ComboBox::from_id_salt(2)
                        .selected_text(format!("{:?}", self.current_config.common.ev_right_click))
                        .show_ui(ui, |ui| {
                            for ev_key in EV_KEYS {
                                ui.selectable_value(
                                    &mut self.current_config.common.ev_right_click,
                                    ev_key,
                                    format!("{:?}", ev_key),
                                );
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Has-Moved Threshold ({}): ",
                        self.current_config.common.has_moved_threshold.to_string()
                    ));
                    if ui.text_edit_singleline(&mut self.input.has_moved).changed() {
                        match self.input.has_moved.parse::<f32>() {
                            Ok(f) => self.current_config.common.has_moved_threshold = f,
                            Err(e) => eprintln!("Has-moved threshold parse error: {e}"),
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Right-Click Wait Time ({}): ",
                        self.current_config.common.right_click_wait_ms.to_string()
                    ));
                    if ui
                        .text_edit_singleline(&mut self.input.right_click_wait)
                        .changed()
                    {
                        match self.input.has_moved.parse::<u64>() {
                            Ok(ms) => self.current_config.common.right_click_wait_ms = ms,
                            Err(e) => eprintln!("Right-click wait time parse error: {e}"),
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Calibration Points: ");
                    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::DARK_GRAY;
                    if ui
                        .button(self.current_config.common.calibration_points.to_string())
                        .clicked()
                    {
                        self.start_calibration(ctx);
                    }
                });
            });
        });
    }

    fn start_calibration(&mut self, ctx: &egui::Context) {
        let calibrator = Calibrator::new(
            &self.device_path,
            ctx,
            #[cfg(feature = "audio")]
            self.sound_manager.get_handle(),
        );
        match self.calibrator_window {
            CalibratorWindowState::Deactivated => {
                self.calibrator_window = CalibratorWindowState::Running(calibrator)
            }
            CalibratorWindowState::Running(_) => {
                panic!("start_calibration: calibrator already running.")
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let old_state = mem::replace(
            &mut self.calibrator_window,
            CalibratorWindowState::Deactivated,
        );

        match old_state {
            CalibratorWindowState::Deactivated => {
                self.process_input(ctx);
                self.draw(ctx);
            }
            CalibratorWindowState::Running(mut calibrator) => {
                let viewport_id = egui::ViewportId(Id::new("calibrator"));
                let viewport_builder = ViewportBuilder::default()
                    .with_title("Calibrator")
                    .with_fullscreen(true);

                let response =
                    ctx.show_viewport_immediate(viewport_id, viewport_builder, |ctx, class| {
                        assert!(class == ViewportClass::Immediate);
                        let response = calibrator.update(ctx);
                        match response {
                            CalibratorWindowResponse::Finish(_) => {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                            }
                            CalibratorWindowResponse::Continue => {}
                        }
                        response
                    });

                match response {
                    CalibratorWindowResponse::Continue => {
                        self.calibrator_window = CalibratorWindowState::Running(calibrator);
                    }
                    CalibratorWindowResponse::Finish(result) => {
                        match result {
                            None => {}
                            Some(calibration_points) => {
                                self.current_config.common.calibration_points = calibration_points;
                            }
                        }
                        calibrator.exit();
                    }
                }
                ctx.request_repaint();
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        log::info!("Shutting down application.");
    }
}
