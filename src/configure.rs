//! The Configure window: lists devices and saves the chosen one to the config
//! file. Runs as its own `--configure` process (started by the tray, which
//! reloads the config when it exits), so the tray's message loop and egui's
//! event loop never have to share a thread.

use std::sync::{Arc, mpsc};

use eframe::egui::{self, Color32, Key, RichText, ViewportCommand};
use egui_extras::{Column, TableBuilder};
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use crate::config::{self, Config};
use crate::listing::{self, HID_CLASS, Rect};
use crate::status::{APP_TITLE, DeviceInfo, describe_problem};
use crate::{device, logging};

const WINDOW_SIZE: egui::Vec2 = egui::vec2(780.0, 500.0);
const WINDOW_ICON: &[u8] = include_bytes!("../assets/window-icon-64.rgba");

/// Open the Configure window and block until it's closed.
pub fn run() -> Result<(), String> {
    let config =
        Config::new(config::default_dir().ok_or("The APPDATA environment variable isn't set.")?);
    let title = format!("{APP_TITLE}! - Configure");

    let mut viewport = egui::ViewportBuilder::default()
        .with_title(&title)
        .with_inner_size(WINDOW_SIZE)
        // Tall enough for the header, footer and a few rows of the list.
        .with_min_inner_size([600.0, 440.0])
        .with_icon(Arc::new(egui::IconData {
            rgba: WINDOW_ICON.to_vec(),
            width: 64,
            height: 64,
        }));
    if let Some(position) = position_near_cursor(WINDOW_SIZE) {
        viewport = viewport.with_position(position);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| Ok(Box::new(ConfigureApp::new(&cc.egui_ctx, config)))),
    )
    .map_err(|err| format!("Couldn't open the Configure window: {err}"))
}

enum Devices {
    Loading(mpsc::Receiver<Result<Vec<DeviceInfo>, String>>),
    Loaded(Vec<DeviceInfo>),
    Failed(String),
}

struct ConfigureApp {
    config: Config,
    devices: Devices,
    show_all: bool,
    filter: String,
    /// Instance ID to save; starts as the configured device.
    selected: Option<String>,
    /// Message shown in the footer (e.g. the config couldn't be saved).
    error: Option<String>,
    /// Whether the list has been checked for a non-HID configured device.
    revealed_selection: bool,
}

impl ConfigureApp {
    fn new(ctx: &egui::Context, config: Config) -> Self {
        let (selected, error) = match config.read() {
            Ok(selected) => (selected, None),
            Err(err) => (
                None,
                Some(format!("Couldn't read {}: {err}", config.path().display())),
            ),
        };
        ConfigureApp {
            config,
            devices: Devices::Loading(load_devices(ctx)),
            show_all: false,
            filter: String::new(),
            selected,
            error,
            revealed_selection: false,
        }
    }

    fn poll_devices(&mut self) {
        let Devices::Loading(receiver) = &self.devices else {
            return;
        };
        self.devices = match receiver.try_recv() {
            Ok(Ok(devices)) => Devices::Loaded(devices),
            Ok(Err(err)) => {
                logging::error(&format!("Listing devices: {err}"));
                Devices::Failed(err)
            }
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                Devices::Failed("The device list couldn't be loaded.".into())
            }
        };

        // If the configured device isn't a HID device it would be hidden by
        // default; show all classes so the current choice is visible.
        if !self.revealed_selection {
            self.revealed_selection = true;
            if let (Devices::Loaded(devices), Some(selected)) = (&self.devices, &self.selected) {
                self.show_all |= devices.iter().any(|d| {
                    listing::same_device(&d.instance_id, selected)
                        && !d.class.eq_ignore_ascii_case(HID_CLASS)
                });
            }
        }
    }

    /// The selected device, if it's in the loaded list.
    fn selected_device(&self) -> Option<&DeviceInfo> {
        let (Devices::Loaded(devices), Some(id)) = (&self.devices, &self.selected) else {
            return None;
        };
        devices
            .iter()
            .find(|d| listing::same_device(&d.instance_id, id))
    }

    fn save(&mut self, ctx: &egui::Context) {
        let Some(id) = self.selected.clone() else {
            return;
        };
        match self.config.write(&id) {
            Ok(()) => {
                logging::info(&format!("Configured device {id}"));
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            // Stay open so the user can retry or cancel.
            Err(err) => {
                let message = format!("Couldn't save to {}: {err}", self.config.path().display());
                logging::error(&message);
                self.error = Some(message);
            }
        }
    }

    fn header(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.add_space(6.0);
        ui.heading("Choose the device the tray icon enables and disables");
        ui.label(
            "Touch screens are usually listed as \"HID-compliant touch screen\". \
             Click a device to select it (its button fills in), then press OK.",
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_all, "Show all device classes");
            ui.separator();
            ui.add(
                egui::TextEdit::singleline(&mut self.filter)
                    .hint_text("Filter")
                    .desired_width(220.0),
            );
            let loading = matches!(self.devices, Devices::Loading(_));
            if ui
                .add_enabled(!loading, egui::Button::new("Refresh"))
                .clicked()
            {
                self.devices = Devices::Loading(load_devices(ctx));
            }
        });
        ui.add_space(6.0);
    }

    fn footer(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Selected device:");
            match &self.selected {
                Some(id) => {
                    ui.add(egui::Label::new(RichText::new(id).monospace()).selectable(true));
                    if let Devices::Loaded(devices) = &self.devices
                        && !devices
                            .iter()
                            .any(|d| listing::same_device(&d.instance_id, id))
                    {
                        ui.weak("(not currently connected)");
                    }
                }
                None => {
                    ui.weak("None");
                }
            }
        });
        // Such devices are hidden from the list, but one may already be
        // saved in the config (as chosen with an older version).
        if self
            .selected_device()
            .is_some_and(|d| !listing::can_toggle(d))
        {
            ui.colored_label(
                Color32::from_rgb(230, 160, 40),
                "Windows doesn't allow this device to be disabled, so clicking the tray icon \
                 won't turn it off. Pick another device.",
            );
        }
        if let Some(error) = &self.error {
            ui.colored_label(Color32::from_rgb(220, 50, 50), error);
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.weak(format!("Version {}", env!("CARGO_PKG_VERSION")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(dialog_button("Cancel")).clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                // Stays blue (just dimmed) until a device is selected, so it's
                // always easy to find.
                if ui
                    .add_enabled(self.selected.is_some(), dialog_button("OK"))
                    .on_disabled_hover_text("Select a device first")
                    .clicked()
                {
                    self.save(ctx);
                }
            });
        });
        ui.add_space(6.0);
    }

    fn device_table(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let devices = match &self.devices {
            Devices::Loading(_) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Loading devices…");
                });
                return;
            }
            Devices::Failed(err) => {
                ui.colored_label(
                    Color32::from_rgb(220, 50, 50),
                    format!("Couldn't list devices: {err}"),
                );
                if ui.button("Try again").clicked() {
                    self.devices = Devices::Loading(load_devices(ctx));
                }
                return;
            }
            Devices::Loaded(devices) => devices,
        };

        let visible = listing::visible_devices(devices, self.show_all, &self.filter);
        if visible.is_empty() {
            ui.weak("No devices match.");
            return;
        }

        // egui labels are selectable text by default, and a selectable label
        // takes the click for itself, so the row never sees it. Rows are
        // the click target here; the footer keeps the chosen ID copyable.
        ui.style_mut().interaction.selectable_labels = false;

        let mut clicked: Option<String> = None;
        let row_height = ui.text_style_height(&egui::TextStyle::Body) + 6.0;
        // Widths are worked out from the current window size every frame
        // (and the columns aren't user-resizable, which would freeze them),
        // so the table always fits the window: the description column takes
        // whatever the others leave.
        let manufacturer_width = (ui.available_width() * 0.22).clamp(110.0, 260.0);
        TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .sense(egui::Sense::click())
            .auto_shrink([false, false])
            .min_scrolled_height(0.0)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::exact(28.0))
            .column(Column::remainder().at_least(120.0).clip(true))
            .column(Column::exact(manufacturer_width).clip(true))
            .column(Column::exact(110.0).clip(true))
            .column(Column::exact(150.0).clip(true))
            .header(row_height, |mut header| {
                for title in ["", "Description", "Manufacturer", "Class", "Status"] {
                    header.col(|ui| {
                        ui.strong(title);
                    });
                }
            })
            .body(|body| {
                body.rows(row_height, visible.len(), |mut row| {
                    let device = visible[row.index()];
                    let is_selected = self
                        .selected
                        .as_deref()
                        .is_some_and(|id| listing::same_device(id, &device.instance_id));
                    row.set_selected(is_selected);
                    // A radio button makes the current choice obvious; it
                    // selects the row just like clicking anywhere else on it.
                    row.col(|ui| {
                        if ui.radio(is_selected, "").clicked() {
                            clicked = Some(device.instance_id.clone());
                        }
                    });
                    row.col(|ui| {
                        ui.label(device.display_name());
                    });
                    row.col(|ui| {
                        ui.label(&device.manufacturer);
                    });
                    row.col(|ui| {
                        ui.label(&device.class);
                    });
                    row.col(|ui| {
                        ui.label(describe_problem(device.problem));
                    });
                    // Full name (it may be cut short) and the instance ID.
                    let tooltip = format!("{}\n{}", device.display_name(), device.instance_id);
                    if row.response().on_hover_text(tooltip).clicked() {
                        clicked = Some(device.instance_id.clone());
                    }
                });
            });

        if let Some(id) = clicked {
            // Clicking the selected row again clears the selection.
            let already = self
                .selected
                .as_deref()
                .is_some_and(|s| listing::same_device(s, &id));
            self.selected = if already { None } else { Some(id) };
            self.error = None;
        }
    }
}

impl eframe::App for ConfigureApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

impl ConfigureApp {
    /// Draw the whole window. Separate from [`eframe::App::ui`] so tests can
    /// drive it headlessly with `egui_kittest`.
    fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.poll_devices();

        let (escape, enter) =
            ctx.input(|i| (i.key_pressed(Key::Escape), i.key_pressed(Key::Enter)));
        if escape {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        } else if enter && !ctx.egui_wants_keyboard_input() {
            self.save(&ctx);
        }

        // The footer is laid out first so it always gets its space: OK and
        // Cancel stay visible however small the window is, and the header
        // and list give way instead.
        egui::Panel::bottom("footer").show(ui, |ui| self.footer(ui, &ctx));
        egui::Panel::top("header").show(ui, |ui| self.header(ui, &ctx));
        egui::CentralPanel::default().show(ui, |ui| self.device_table(ui, &ctx));
    }
}

/// The blue OK / Cancel buttons: larger than egui's default and filled, so
/// they stand out against the dark theme.
fn dialog_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text).color(Color32::WHITE).strong())
        .fill(Color32::from_rgb(0, 103, 192))
        .min_size(egui::vec2(88.0, 30.0))
}

/// List devices on a background thread so the window opens immediately.
fn load_devices(ctx: &egui::Context) -> mpsc::Receiver<Result<Vec<DeviceInfo>, String>> {
    let (sender, receiver) = mpsc::channel();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let _ = sender.send(device::list_devices().map_err(|err| err.to_string()));
        ctx.request_repaint();
    });
    receiver
}

/// Where to open the window (in points): above-left of the cursor, which is
/// next to the tray icon the user just clicked, kept on that monitor.
fn position_near_cursor(inner_size: egui::Vec2) -> Option<egui::Pos2> {
    unsafe {
        let mut cursor = POINT::default();
        GetCursorPos(&mut cursor).ok()?;
        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return None;
        }
        let (mut dpi_x, mut dpi_y) = (96, 96);
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        let scale = dpi_x as f32 / 96.0;

        // Allow for the title bar and borders around the inner size.
        let outer = ((inner_size.x + 16.0) * scale, (inner_size.y + 40.0) * scale);
        let work = info.rcWork;
        let (x, y) = listing::place_near(
            (cursor.x, cursor.y),
            (outer.0.round() as i32, outer.1.round() as i32),
            Rect {
                left: work.left,
                top: work.top,
                right: work.right,
                bottom: work.bottom,
            },
        );
        Some(egui::pos2(x as f32 / scale, y as f32 / scale))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    const PEN: &str = r"HID\ELAN2D25&COL02\5&2B77D6B&0&0001";
    const TOUCH: &str = r"HID\ELAN2D25&COL01\5&2B77D6B&0&0000";

    fn device(id: &str, description: &str) -> DeviceInfo {
        DeviceInfo {
            instance_id: id.into(),
            description: description.into(),
            manufacturer: "ELAN".into(),
            class: HID_CLASS.into(),
            problem: 0,
            can_disable: true,
        }
    }

    /// The window with a fixed device list and a config in a temp folder.
    fn harness(dir: &std::path::Path) -> Harness<'static, ConfigureApp> {
        let app = ConfigureApp {
            config: Config::new(dir),
            devices: Devices::Loaded(vec![
                device(TOUCH, "HID-compliant touch screen"),
                device(PEN, "HID-compliant pen"),
            ]),
            show_all: false,
            filter: String::new(),
            selected: None,
            error: None,
            revealed_selection: true,
        };
        let mut harness = Harness::builder()
            .with_size(WINDOW_SIZE)
            .build_ui_state(|ui, app: &mut ConfigureApp| app.show(ui), app);
        harness.run();
        harness
    }

    #[test]
    fn clicking_a_rows_text_selects_it() {
        let temp = tempfile::tempdir().unwrap();
        let mut harness = harness(temp.path());

        // Clicking right on the label text, which used to start a text
        // selection instead of selecting the row.
        harness.get_by_label("HID-compliant pen").click();
        harness.run();
        assert_eq!(harness.state().selected.as_deref(), Some(PEN));

        harness.get_by_label("HID-compliant touch screen").click();
        harness.run();
        assert_eq!(harness.state().selected.as_deref(), Some(TOUCH));
    }

    #[test]
    fn clicking_other_columns_selects_the_row() {
        let temp = tempfile::tempdir().unwrap();
        let mut harness = harness(temp.path());

        // Rows are sorted by description: pen first, then the touch screen.
        harness.get_all_by_label("ELAN").nth(1).unwrap().click();
        harness.run();
        assert_eq!(harness.state().selected.as_deref(), Some(TOUCH));
    }

    #[test]
    fn clicking_a_radio_button_selects_its_row() {
        let temp = tempfile::tempdir().unwrap();
        let mut harness = harness(temp.path());

        // Rows are sorted by description: pen first, then the touch screen.
        harness
            .get_all_by_role(egui::accesskit::Role::RadioButton)
            .nth(1)
            .unwrap()
            .click();
        harness.run();
        assert_eq!(harness.state().selected.as_deref(), Some(TOUCH));
    }

    #[test]
    fn clicking_the_selected_row_again_clears_it() {
        let temp = tempfile::tempdir().unwrap();
        let mut harness = harness(temp.path());

        harness.get_by_label("HID-compliant pen").click();
        harness.run();
        harness.get_by_label("HID-compliant pen").click();
        harness.run();
        assert_eq!(harness.state().selected, None);
    }

    #[test]
    fn devices_that_cannot_be_disabled_are_hidden() {
        let temp = tempfile::tempdir().unwrap();
        let mut harness = harness(temp.path());
        if let Devices::Loaded(devices) = &mut harness.state_mut().devices {
            devices[1].can_disable = false; // the pen
        }
        harness.run();

        assert!(harness.query_by_label("HID-compliant pen").is_none());
        assert!(
            harness
                .query_by_label("HID-compliant touch screen")
                .is_some()
        );
    }

    #[test]
    fn warns_when_the_saved_device_cannot_be_disabled() {
        let temp = tempfile::tempdir().unwrap();
        let mut harness = harness(temp.path());
        let warning = "Windows doesn't allow this device to be disabled";

        harness.get_by_label("HID-compliant pen").click();
        harness.run();
        assert!(harness.query_by_label_contains(warning).is_none());

        // E.g. a device picked with an older version: hidden from the list,
        // but still selected, so say why it won't work.
        if let Devices::Loaded(devices) = &mut harness.state_mut().devices {
            devices.iter_mut().for_each(|d| d.can_disable = false);
        }
        harness.run();
        assert!(harness.query_by_label("HID-compliant pen").is_none());
        assert!(harness.query_by_label_contains(warning).is_some());
        assert_eq!(harness.state().selected.as_deref(), Some(PEN));
    }

    #[test]
    fn ok_saves_the_selected_device() {
        let temp = tempfile::tempdir().unwrap();
        let mut harness = harness(temp.path());

        harness.get_by_label("HID-compliant pen").click();
        harness.run();
        harness.get_by_label("OK").click();
        harness.run();

        let saved = Config::new(temp.path()).read().unwrap();
        assert_eq!(saved.as_deref(), Some(PEN));
        assert_eq!(harness.state().error, None);
    }

    #[test]
    fn ok_does_nothing_without_a_selection() {
        let temp = tempfile::tempdir().unwrap();
        let mut harness = harness(temp.path());

        harness.get_by_label("OK").click();
        harness.run();
        assert_eq!(Config::new(temp.path()).read().unwrap(), None);
    }

    #[test]
    fn failed_save_keeps_the_window_open_with_an_error() {
        let temp = tempfile::tempdir().unwrap();
        // A directory where the config file should go makes the write fail.
        std::fs::create_dir_all(temp.path().join(config::CONFIG_FILE_NAME)).unwrap();
        let mut harness = harness(temp.path());

        harness.get_by_label("HID-compliant pen").click();
        harness.run();
        harness.get_by_label("OK").click();
        harness.run();

        let error = harness.state().error.clone().expect("error shown");
        assert!(error.starts_with("Couldn't save to"), "{error}");
        assert_eq!(harness.state().selected.as_deref(), Some(PEN));
    }
}
