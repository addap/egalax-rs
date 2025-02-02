#[cfg(feature = "audio")]
mod audio;
mod calibrate;
mod evdev_events;

use egui::{vec2, Color32, FontId, Id, Key, TextStyle, Theme, ViewportBuilder, ViewportClass};
use std::{fs::OpenOptions, mem, path::PathBuf};

use calibrate::Calibrator;
use egalax_rs::{
    config::{self, SerializedConfig},
    geo::AABB,
};
use evdev_events::EV_KEYS;

const FOOTER_STYLE: &str = "footer";
const CONTENT_OFFSET: f32 = 0.3;

struct Input {
    has_moved: String,
    right_click_wait: String,
}

impl Input {
    fn new(config_file: &SerializedConfig) -> Self {
        Self {
            has_moved: config_file.common.has_moved_threshold.to_string(),
            right_click_wait: config_file.common.right_click_wait_ms.to_string(),
        }
    }
}

struct StaticData {
    quit_save_msg: String,
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
    current_config: SerializedConfig,
    original_config: SerializedConfig,
    input: Input,
    monitors: Vec<String>,
    device_path: PathBuf,
    config_path: PathBuf,
    calibrator_window: CalibratorWindowState,
    static_data: StaticData,
    #[cfg(feature = "audio")]
    sound_manager: audio::SoundManager,
}

impl App {
    pub fn new(
        device_path: PathBuf,
        config_path: PathBuf,
        original_config: SerializedConfig,
        monitors: Vec<String>,
        cc: &eframe::CreationContext<'_>,
    ) -> Self {
        cc.egui_ctx
            .options_mut(|options| options.fallback_theme = Theme::Light);

        let current_config = original_config.clone();
        let input = Input::new(&original_config);
        let static_data = StaticData {
            quit_save_msg: format!("Quit & save to \"{}\"", config_path.display()),
        };

        #[cfg(feature = "audio")]
        let sound_manager = audio::SoundManager::init().unwrap();

        Self {
            current_config,
            original_config,
            input,
            monitors,
            device_path,
            config_path,
            calibrator_window: CalibratorWindowState::Deactivated,
            static_data,
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
            let mut config_file = OpenOptions::new()
                .write(true)
                .open(&self.config_path)
                .expect("TODO unable to open config file");
            if let Err(e) = self.current_config.save_file(&mut config_file) {
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
                ("Enter", &self.static_data.quit_save_msg),
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
