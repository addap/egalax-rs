#[cfg(feature = "audio")]
mod audio;
mod calibrate;

use const_format::formatcp;
use egui::{mutex::Mutex, vec2, Color32, FontId, Id, Key, TextStyle, Theme, ViewportBuilder};
use std::{
    mem,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::Arc,
};

use calibrate::Calibrator;
use egalax_rs::{
    config::{self, ConfigFile},
    geo::AABB,
};

const CONFIG_FILE_PATH: &str = "./config.toml";
const FOOTER_STYLE: &str = "footer";
const CONTENT_OFFSET: f32 = 0.3;

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

enum CalibratorWindow {
    Deactivated,
    Running(Calibrator),
    Finished(Option<AABB>),
}

pub struct App {
    current_config: ConfigFile,
    original_config: ConfigFile,
    input: Input,
    monitors: Vec<String>,
    device_path: PathBuf,
    calibrator_state: Arc<Mutex<CalibratorWindow>>,
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
            calibrator_state: Arc::new(Mutex::new(CalibratorWindow::Deactivated)),
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
                    egui::ComboBox::from_label("")
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
                // left-click event
                // right-click event
                // right-click wait time (ms)
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
        let mut guard = self.calibrator_state.lock();
        let old_state = mem::replace(guard.deref_mut(), CalibratorWindow::Running(calibrator));

        match old_state {
            CalibratorWindow::Deactivated => {}
            CalibratorWindow::Running(_) => {
                panic!("start_calibration: calibrator already running.")
            }
            CalibratorWindow::Finished(_) => {
                panic!("start_calibration: calibrator already finished.")
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut guard = self.calibrator_state.lock();
        match guard.deref() {
            CalibratorWindow::Deactivated => {
                drop(guard);
                self.process_input(ctx);
                self.draw(ctx);
            }
            CalibratorWindow::Running(_) => {
                let calibrator_state = self.calibrator_state.clone();
                let viewport_id = egui::ViewportId(Id::new("calibrator"));
                let viewport_builder = ViewportBuilder::default()
                    .with_title("Calibrator")
                    .with_fullscreen(true);

                ctx.show_viewport_deferred(viewport_id, viewport_builder, move |ctx, class| {
                    Calibrator::update(&calibrator_state, ctx, class);
                });
            }
            CalibratorWindow::Finished(result) => {
                if let Some(calibration_points) = result {
                    self.current_config.common.calibration_points = *calibration_points;
                }
                *guard = CalibratorWindow::Deactivated;
                // Need to request repaint (or call draw function) to repaint GUI.
                ctx.request_repaint();
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        log::info!("Shutting down application.");
    }
}
