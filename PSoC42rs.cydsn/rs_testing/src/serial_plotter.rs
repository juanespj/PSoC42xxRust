use eframe::egui;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use async_trait::async_trait;

const SCALE: f64 = 65536.0;

#[derive(Debug, Clone)]
pub struct DataPoint {
    pub value: f64,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub enum Control {
    Start,
    Stop,
    Reset,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AsyncSerialPort: Send {
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
    async fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
    async fn flush(&mut self) -> std::io::Result<()>;
}

#[async_trait]
impl AsyncSerialPort for tokio_serial::SerialStream {
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        use tokio::io::AsyncReadExt;
        AsyncReadExt::read(self, buf).await
    }

    async fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        use tokio::io::AsyncWriteExt;
        AsyncWriteExt::write(self, buf).await
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        use tokio::io::AsyncWriteExt;
        AsyncWriteExt::flush(self).await
    }
}

// Reader task
pub async fn reader_task<P: AsyncSerialPort + 'static>(
    mut port: P,
    data_tx: mpsc::Sender<DataPoint>,
    log_tx: mpsc::Sender<String>,
    mut control_rx: mpsc::Receiver<Control>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0u8; 64];
    let mut running = false;

    loop {
        tokio::select! {
            Some(cmd) = control_rx.recv() => {
                match cmd {
                    Control::Start => {
                        running = true;
                        let _ = log_tx.send("Started reading".to_string()).await;
                    }
                    Control::Stop => {
                        running = false;
                        let _ = log_tx.send("Stopped reading".to_string()).await;
                    }
                    Control::Reset => {
                        let _ = log_tx.send("Reset".to_string()).await;
                    }
                }
            }

            result = port.read(&mut buffer), if running => {
                match result {
                    Ok(n) if n > 0 => {
                        let data_str = String::from_utf8_lossy(&buffer[..n]);

                        if let Ok(value) = data_str.trim().parse::<f64>() {
                            let dp = DataPoint {
                                value,
                                timestamp: std::time::SystemTime::now(),
                            };

                            if data_tx.send(dp).await.is_err() {
                                break;
                            }
                        } else {
                            let _ = log_tx.send(format!("Parse error: {}", data_str)).await;
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        let _ = log_tx.send(format!("Read error: {}", e)).await;
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
            }
        }
    }

    Ok(())
}

// Shared application state
#[derive(Clone)]
pub struct AppState {
    data_points: Arc<Mutex<VecDeque<DataPoint>>>,
    logs: Arc<Mutex<VecDeque<String>>>,
    is_running: Arc<Mutex<bool>>,
    max_points: usize,
}

impl AppState {
    pub fn new(max_points: usize) -> Self {
        Self {
            data_points: Arc::new(Mutex::new(VecDeque::new())),
            logs: Arc::new(Mutex::new(VecDeque::new())),
            is_running: Arc::new(Mutex::new(false)),
            max_points,
        }
    }

    pub fn add_data_point(&self, dp: DataPoint) {
        let mut points = self.data_points.lock().unwrap();
        points.push_back(dp);
        while points.len() > self.max_points {
            points.pop_front();
        }
    }

    pub fn add_log(&self, log: String) {
        let mut logs = self.logs.lock().unwrap();
        logs.push_back(log);
        while logs.len() > 100 {
            logs.pop_front();
        }
    }

    fn clear_data(&self) {
        self.data_points.lock().unwrap().clear();
    }

    fn set_running(&self, running: bool) {
        *self.is_running.lock().unwrap() = running;
    }

    fn is_running(&self) -> bool {
        *self.is_running.lock().unwrap()
    }
}

fn send_cmd(command_tx: &mpsc::Sender<String>, cmd: &str, state: &AppState) {
    let _ = command_tx.try_send(cmd.to_string());
    state.add_log(format!(">> {}", cmd));
}

// Main egui application
pub struct SerialPlotterApp {
    state: AppState,
    control_tx: mpsc::Sender<Control>,
    command_tx: mpsc::Sender<String>,
    command_input: String,
    selected_port: String,
    baud_rate: u32,
    available_ports: Vec<String>,

    // Tab
    selected_tab: usize,

    // ADRC tuning params (in physical units)
    adrc_w0: f64,
    adrc_wc: f64,
    adrc_b0: f64,
    adrc_mode: usize,
    target_position: i32,
    speed_setpoint: f64,

    // Encoder filter gains (physical)
    encoder_ga: f64,
    encoder_gb: f64,
    encoder_gc: f64,
}

impl SerialPlotterApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        control_tx: mpsc::Sender<Control>,
        command_tx: mpsc::Sender<String>,
    ) -> Self {
        let available_ports = tokio_serial::available_ports()
            .unwrap_or_default()
            .iter()
            .map(|p| p.port_name.clone())
            .collect();

        Self {
            state: AppState::new(1000),
            control_tx,
            command_tx,
            command_input: String::new(),
            selected_port: String::new(),
            baud_rate: 115200,
            available_ports,
            selected_tab: 1,
            adrc_w0: 1000.0,
            adrc_wc: 200.0,
            adrc_b0: 1.0,
            adrc_mode: 0,
            target_position: 0,
            speed_setpoint: 1000.0,
            encoder_ga: 0.33,
            encoder_gb: 0.01,
            encoder_gc: 0.00021,
        }
    }
}

impl eframe::App for SerialPlotterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        // ── Top panel: port controls + tab bar ──
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Port:");
                egui::ComboBox::from_id_salt("port_selector")
                    .selected_text(&self.selected_port)
                    .show_ui(ui, |ui| {
                        for port in &self.available_ports {
                            ui.selectable_value(&mut self.selected_port, port.clone(), port);
                        }
                    });

                ui.label("Baud:");
                ui.add(egui::DragValue::new(&mut self.baud_rate).speed(100));

                if ui.button("Refresh Ports").clicked() {
                    self.available_ports = tokio_serial::available_ports()
                        .unwrap_or_default()
                        .iter()
                        .map(|p| p.port_name.clone())
                        .collect();
                }

                ui.separator();

                ui.label("Command:");
                ui.text_edit_singleline(&mut self.command_input);
                if ui.button("Send").clicked() && !self.command_input.is_empty() {
                    let _ = self.command_tx.try_send(self.command_input.clone());
                    self.command_input.clear();
                }
            });

            ui.separator();

            // Tab bar
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.selected_tab, 0, "📊 Plot");
                ui.selectable_value(&mut self.selected_tab, 1, "🎛️ Motor Tuning");
            });
        });

        // ── Bottom panel: logs ──
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.heading("Logs");
            egui::ScrollArea::vertical()
                .max_height(100.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    let logs = self.state.logs.lock().unwrap();
                    for log in logs.iter() {
                        ui.label(log);
                    }
                });
        });

        // ── Central panel: tab content ──
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.selected_tab {
                0 => self.plot_tab(ui),
                1 => self.tuning_tab(ui),
                _ => {}
            }
        });
    }
}

// ─── Tab 0: Plot ──────────────────────────────────────────────
impl SerialPlotterApp {
    fn plot_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Serial Data Plot");

        ui.horizontal(|ui| {
            let is_running = self.state.is_running();

            if ui
                .button(if is_running { "⏸ Stop" } else { "▶ Start" })
                .clicked()
            {
                let _ = self.control_tx.try_send(if is_running {
                    Control::Stop
                } else {
                    Control::Start
                });
                self.state.set_running(!is_running);
            }

            if ui.button("🔄 Reset Plot").clicked() {
                let _ = self.control_tx.try_send(Control::Reset);
                self.state.clear_data();
            }
        });

        let points = self.state.data_points.lock().unwrap();

        if points.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No data yet. Press Start to begin collecting data.");
            });
        } else {
            use egui_plot::{Line, Plot, PlotPoints};

            let plot_points: PlotPoints = points
                .iter()
                .enumerate()
                .map(|(i, dp)| [i as f64, dp.value])
                .collect();

            let line = Line::new(plot_points);

            Plot::new("data_plot").view_aspect(2.0).show(ui, |plot_ui| {
                plot_ui.line(line);
            });

            ui.horizontal(|ui| {
                if let Some(latest) = points.back() {
                    ui.label(format!("Latest: {:.2}", latest.value));
                }

                let values: Vec<f64> = points.iter().map(|p| p.value).collect();
                if !values.is_empty() {
                    let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                    let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                    let avg = values.iter().sum::<f64>() / values.len() as f64;

                    ui.separator();
                    ui.label(format!("Min: {:.2}", min));
                    ui.label(format!("Max: {:.2}", max));
                    ui.label(format!("Avg: {:.2}", avg));
                }
            });
        }
    }
}

// ─── Tab 1: Motor Tuning ──────────────────────────────────────
impl SerialPlotterApp {
    fn tuning_tab(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("tuning_grid")
            .striped(true)
            .min_col_width(120.0)
            .show(ui, |ui| {
                // ── ADRC Section ──
                ui.label("ADRC Control");
                ui.end_row();

                ui.label("w0 (observer bandwidth):");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Slider::new(&mut self.adrc_w0, 100.0..=50000.0)
                            .logarithmic(true)
                    );
                    ui.label(format!("{:.0} rad/s", self.adrc_w0));
                    if ui.button("Send").clicked() {
                        let val = (self.adrc_w0 * SCALE) as u64;
                        send_cmd(
                            &self.command_tx,
                            &format!(">w{},", val),
                            &self.state,
                        );
                    }
                });
                ui.end_row();

                ui.label("wc (controller bandwidth):");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Slider::new(&mut self.adrc_wc, 10.0..=5000.0)
                            .logarithmic(true),
                    );
                    ui.label(format!("{:.0} rad/s", self.adrc_wc));
                    if ui.button("Send").clicked() {
                        let val = (self.adrc_wc * SCALE) as u64;
                        send_cmd(
                            &self.command_tx,
                            &format!(">v{},", val),
                            &self.state,
                        );
                    }
                });
                ui.end_row();

                ui.label("b0 (plant gain):");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Slider::new(&mut self.adrc_b0, 0.1..=10.0)
                            .logarithmic(true),
                    );
                    ui.label(format!("{:.3}", self.adrc_b0));
                    if ui.button("Send").clicked() {
                        let val = (self.adrc_b0 * SCALE) as u64;
                        send_cmd(
                            &self.command_tx,
                            &format!(">o{},", val),
                            &self.state,
                        );
                    }
                });
                ui.end_row();

                ui.label("ADRC Mode:");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("adrc_mode")
                        .selected_text(match self.adrc_mode {
                            0 => "Off",
                            1 => "Speed",
                            2 => "Position",
                            _ => "?",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.adrc_mode, 0, "Off");
                            ui.selectable_value(&mut self.adrc_mode, 1, "Speed");
                            ui.selectable_value(&mut self.adrc_mode, 2, "Position");
                        });
                    if ui.button("Set").clicked() {
                        send_cmd(
                            &self.command_tx,
                            &format!(">m{},", self.adrc_mode),
                            &self.state,
                        );
                    }
                });
                ui.end_row();

                ui.label("Target Position:");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.target_position).speed(10));
                    if ui.button("Set").clicked() {
                        send_cmd(
                            &self.command_tx,
                            &format!(">q{},", self.target_position),
                            &self.state,
                        );
                    }
                });
                ui.end_row();

                ui.label("Speed Setpoint:");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Slider::new(&mut self.speed_setpoint, 0.0..=10000.0)
                            .logarithmic(true),
                    );
                    ui.label(format!("{:.0} Hz", self.speed_setpoint));
                    if ui.button("Set").clicked() {
                        send_cmd(
                            &self.command_tx,
                            &format!(">p{},", self.speed_setpoint as u64),
                            &self.state,
                        );
                    }
                });
                ui.end_row();

                ui.label("");
                ui.end_row();

                // ── Encoder Filter Section ──
                ui.label("Encoder Filter");
                ui.end_row();

                for (label, val, min, max, step, send_char) in [
                    ("g_a (velocity):", &mut self.encoder_ga, 0.0001, 1.0, 0.00001, 'a'),
                    ("g_b (velocity2):", &mut self.encoder_gb, 0.0001, 0.01, 0.0001, 'b'),
                    ("g_c (accel):", &mut self.encoder_gc, 0.00001, 0.001, 0.00001, 'c'),
                ] {
                    ui.label(label);
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Slider::new(val, min..=max)
                                .logarithmic(true)
                                .step_by(step),
                        );
                        if ui.button("Send").clicked() {
                            let bits = (*val * SCALE) as u64;
                            send_cmd(
                                &self.command_tx,
                                &format!(">{}{},", send_char, bits),
                                &self.state,
                            );
                        }
                    });
                    ui.end_row();
                }

                ui.label("");
                ui.end_row();

                // ── Motion Controls ──
                ui.label("Motion Controls");
                ui.end_row();

                ui.horizontal(|ui| {
                    if ui.button("▶ Start Move").clicked() {
                        send_cmd(&self.command_tx, "r", &self.state);
                    }
                    if ui.button("▶ Start Speed").clicked() {
                        send_cmd(&self.command_tx, "s", &self.state);
                    }
                    if ui.button("⏹ Stop").clicked() {
                        send_cmd(&self.command_tx, "t", &self.state);
                    }
                    if ui.button("✕ Kill").clicked() {
                        send_cmd(&self.command_tx, "k", &self.state);
                    }
                });
                ui.end_row();

                ui.horizontal(|ui| {
                    if ui.button("↺ Reset Encoder").clicked() {
                        send_cmd(&self.command_tx, "z", &self.state);
                    }
                    if ui.button("⇄ Toggle Dir").clicked() {
                        send_cmd(&self.command_tx, "d", &self.state);
                    }
                    if ui.button("🐌 Low Speed (500 Hz)").clicked() {
                        send_cmd(
                            &self.command_tx,
                            &format!(">p{},", 500u64),
                            &self.state,
                        );
                    }
                    if ui.button("🏃 High Speed (2000 Hz)").clicked() {
                        send_cmd(
                            &self.command_tx,
                            &format!(">p{},", 2000u64),
                            &self.state,
                        );
                    }
                });
                ui.end_row();

                ui.label("");
                ui.end_row();

                // ── Presets ──
                ui.label("ADRC Presets");
                ui.end_row();

                ui.horizontal(|ui| {
                    if ui.button("Conservative").clicked() {
                        self.adrc_w0 = 500.0;
                        self.adrc_wc = 100.0;
                        self.adrc_b0 = 1.0;
                        let w0 = (500.0 * SCALE) as u64;
                        let wc = (100.0 * SCALE) as u64;
                        let b0 = (1.0 * SCALE) as u64;
                        let l = &self.state;
                        send_cmd(&self.command_tx, &format!(">w{},", w0), l);
                        send_cmd(&self.command_tx, &format!(">v{},", wc), l);
                        send_cmd(&self.command_tx, &format!(">o{},", b0), l);
                    }
                    if ui.button("Aggressive").clicked() {
                        self.adrc_w0 = 3000.0;
                        self.adrc_wc = 600.0;
                        self.adrc_b0 = 1.0;
                        let w0 = (3000.0 * SCALE) as u64;
                        let wc = (600.0 * SCALE) as u64;
                        let b0 = (1.0 * SCALE) as u64;
                        let l = &self.state;
                        send_cmd(&self.command_tx, &format!(">w{},", w0), l);
                        send_cmd(&self.command_tx, &format!(">v{},", wc), l);
                        send_cmd(&self.command_tx, &format!(">o{},", b0), l);
                    }
                    if ui.button("Very Aggressive").clicked() {
                        self.adrc_w0 = 10000.0;
                        self.adrc_wc = 2000.0;
                        self.adrc_b0 = 1.0;
                        let w0 = (10000.0 * SCALE) as u64;
                        let wc = (2000.0 * SCALE) as u64;
                        let b0 = (1.0 * SCALE) as u64;
                        let l = &self.state;
                        send_cmd(&self.command_tx, &format!(">w{},", w0), l);
                        send_cmd(&self.command_tx, &format!(">v{},", wc), l);
                        send_cmd(&self.command_tx, &format!(">o{},", b0), l);
                    }
                });
                ui.end_row();
            });
    }
}
