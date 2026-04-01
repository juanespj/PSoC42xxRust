use eframe::egui;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

// Your serial trait and implementations (from previous code)
use async_trait::async_trait;

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

// Main egui application
pub struct SerialPlotterApp {
    state: AppState,
    control_tx: mpsc::Sender<Control>,
    command_tx: mpsc::Sender<String>,
    command_input: String,
    selected_port: String,
    baud_rate: u32,
    available_ports: Vec<String>,
}

impl SerialPlotterApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        control_tx: mpsc::Sender<Control>,
        command_tx: mpsc::Sender<String>,
    ) -> Self {
        // Get available serial ports
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
            baud_rate: 9600,
            available_ports,
        }
    }
}

impl eframe::App for SerialPlotterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request continuous repaints for real-time updates
        ctx.request_repaint();

        // Top panel - controls
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
            });

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

                if ui.button("🔄 Reset").clicked() {
                    let _ = self.control_tx.try_send(Control::Reset);
                    self.state.clear_data();
                }

                ui.separator();

                ui.label("Command:");
                ui.text_edit_singleline(&mut self.command_input);
                if ui.button("Send").clicked() && !self.command_input.is_empty() {
                    let _ = self.command_tx.try_send(self.command_input.clone());
                    self.command_input.clear();
                }
            });
        });

        // Bottom panel - logs
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

        // Central panel - plot
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Serial Data Plot");

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

                // Statistics
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
        });
    }
}
