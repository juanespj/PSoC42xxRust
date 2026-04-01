use async_trait::async_trait;
use eframe::egui;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

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

// Writer task
pub async fn writer_task<P: AsyncSerialPort + 'static>(
    mut port: P,
    mut command_rx: mpsc::Receiver<String>,
    log_tx: mpsc::Sender<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    while let Some(command) = command_rx.recv().await {
        match port.write(command.as_bytes()).await {
            Ok(n) => {
                port.flush().await?;
                let _ = log_tx.send(format!("Wrote {} bytes: {}", n, command)).await;
            }
            Err(e) => {
                let _ = log_tx.send(format!("Write error: {}", e)).await;
                break;
            }
        }
    }

    Ok(())
}

// Shared application state
#[derive(Clone)]
struct AppState {
    data_points: Arc<Mutex<VecDeque<DataPoint>>>,
    logs: Arc<Mutex<VecDeque<String>>>,
    is_running: Arc<Mutex<bool>>,
    is_connected: Arc<Mutex<bool>>,
    control_tx: Arc<Mutex<Option<mpsc::Sender<Control>>>>,
    command_tx: Arc<Mutex<Option<mpsc::Sender<String>>>>,
    max_points: usize,
}

impl AppState {
    fn new(max_points: usize) -> Self {
        Self {
            data_points: Arc::new(Mutex::new(VecDeque::new())),
            logs: Arc::new(Mutex::new(VecDeque::new())),
            is_running: Arc::new(Mutex::new(false)),
            is_connected: Arc::new(Mutex::new(false)),
            control_tx: Arc::new(Mutex::new(None)),
            command_tx: Arc::new(Mutex::new(None)),
            max_points,
        }
    }

    fn add_data_point(&self, dp: DataPoint) {
        let mut points = self.data_points.lock().unwrap();
        points.push_back(dp);
        while points.len() > self.max_points {
            points.pop_front();
        }
    }

    fn add_log(&self, log: String) {
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

    fn set_connected(&self, connected: bool) {
        *self.is_connected.lock().unwrap() = connected;
    }

    fn is_connected(&self) -> bool {
        *self.is_connected.lock().unwrap()
    }

    fn set_control_channel(&self, tx: mpsc::Sender<Control>) {
        *self.control_tx.lock().unwrap() = Some(tx);
    }

    fn set_command_channel(&self, tx: mpsc::Sender<String>) {
        *self.command_tx.lock().unwrap() = Some(tx);
    }

    fn send_control(&self, cmd: Control) {
        if let Some(tx) = self.control_tx.lock().unwrap().as_ref() {
            let _ = tx.try_send(cmd);
        }
    }

    fn send_command(&self, cmd: String) {
        if let Some(tx) = self.command_tx.lock().unwrap().as_ref() {
            let _ = tx.try_send(cmd);
        }
    }
}

// Main egui application
struct SerialPlotterApp {
    state: AppState,
    connect_tx: mpsc::Sender<(String, u32)>,
    command_input: String,
    selected_port: String,
    baud_rate: u32,
    available_ports: Vec<String>,
}

impl SerialPlotterApp {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        state: AppState,
        connect_tx: mpsc::Sender<(String, u32)>,
    ) -> Self {
        // Get available serial ports
        let available_ports = tokio_serial::available_ports()
            .unwrap_or_default()
            .iter()
            .map(|p| p.port_name.clone())
            .collect();

        Self {
            state,
            connect_tx,
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

                let is_connected = self.state.is_connected();

                if !is_connected {
                    if ui.button("Connect").clicked() && !self.selected_port.is_empty() {
                        let _ = self
                            .connect_tx
                            .try_send((self.selected_port.clone(), self.baud_rate));
                        self.state.add_log(format!(
                            "Connecting to {} at {} baud...",
                            self.selected_port, self.baud_rate
                        ));
                    }
                } else {
                    ui.label("✓ Connected");
                }
            });

            if self.state.is_connected() {
                ui.horizontal(|ui| {
                    let is_running = self.state.is_running();

                    if ui
                        .button(if is_running { "⏸ Stop" } else { "▶ Start" })
                        .clicked()
                    {
                        self.state.send_control(if is_running {
                            Control::Stop
                        } else {
                            Control::Start
                        });
                        self.state.set_running(!is_running);
                    }

                    if ui.button("🔄 Reset").clicked() {
                        self.state.send_control(Control::Reset);
                        self.state.clear_data();
                    }

                    ui.separator();

                    ui.label("Command:");
                    ui.text_edit_singleline(&mut self.command_input);
                    if ui.button("Send").clicked() && !self.command_input.is_empty() {
                        self.state.send_command(self.command_input.clone());
                        self.command_input.clear();
                    }
                });
            }
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
                    ui.label(
                        "No data yet. Connect to a port and press Start to begin collecting data.",
                    );
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
                        ui.label(format!("Points: {}", values.len()));
                    }
                });
            }
        });
    }
}
use tokio_serial::SerialPortBuilderExt;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create app state
    let app_state = AppState::new(1000);

    // Create channels for connection management
    let (connect_tx, mut connect_rx) = mpsc::channel::<(String, u32)>(10);

    // Clone state for background thread
    let state_clone = app_state.clone();

    // Spawn tokio background thread
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        runtime.block_on(async {
            // Data and log channels
            let (data_tx, mut data_rx) = mpsc::channel::<DataPoint>(100);
            let (log_tx, mut log_rx) = mpsc::channel::<String>(100);

            // Data receiver task
            let state_clone_data = state_clone.clone();
            tokio::spawn(async move {
                while let Some(dp) = data_rx.recv().await {
                    state_clone_data.add_data_point(dp);
                }
            });

            // Log receiver task
            let state_clone_log = state_clone.clone();
            tokio::spawn(async move {
                while let Some(log) = log_rx.recv().await {
                    state_clone_log.add_log(log);
                }
            });

            // Connection handler
            while let Some((port_name, baud_rate)) = connect_rx.recv().await {
                let data_tx = data_tx.clone();
                let log_tx_clone = log_tx.clone();
                let state_clone_conn = state_clone.clone();

                // Try to open the serial port
                match tokio_serial::new(&port_name, baud_rate).open_native_async() {
                    Ok(port) => {
                        log_tx_clone
                            .send(format!("Connected to {}", port_name))
                            .await
                            .ok();
                        state_clone_conn.set_connected(true);

                        // Create control and command channels for this connection
                        let (control_tx, control_rx) = mpsc::channel::<Control>(10);
                        let (command_tx, command_rx) = mpsc::channel::<String>(10);

                        // Store channels in state
                        state_clone_conn.set_control_channel(control_tx);
                        state_clone_conn.set_command_channel(command_tx);

                        // Split the port for reading and writing
                        let (reader, writer) = tokio::io::split(port);

                        // Create wrapper types
                        struct SerialReader(tokio::io::ReadHalf<tokio_serial::SerialStream>);
                        struct SerialWriter(tokio::io::WriteHalf<tokio_serial::SerialStream>);

                        #[async_trait]
                        impl AsyncSerialPort for SerialReader {
                            async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                                use tokio::io::AsyncReadExt;
                                self.0.read(buf).await
                            }

                            async fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                                Err(std::io::Error::new(
                                    std::io::ErrorKind::Unsupported,
                                    "write not supported",
                                ))
                            }

                            async fn flush(&mut self) -> std::io::Result<()> {
                                Ok(())
                            }
                        }

                        #[async_trait]
                        impl AsyncSerialPort for SerialWriter {
                            async fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                                Err(std::io::Error::new(
                                    std::io::ErrorKind::Unsupported,
                                    "read not supported",
                                ))
                            }

                            async fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                                use tokio::io::AsyncWriteExt;
                                self.0.write(buf).await
                            }

                            async fn flush(&mut self) -> std::io::Result<()> {
                                use tokio::io::AsyncWriteExt;
                                self.0.flush().await
                            }
                        }

                        // Spawn reader task
                        tokio::spawn(async move {
                            let _ = reader_task(
                                SerialReader(reader),
                                data_tx,
                                log_tx_clone.clone(),
                                control_rx,
                            )
                            .await;
                            state_clone_conn.set_connected(false);
                            state_clone_conn.set_running(false);
                            log_tx_clone.send("Disconnected".to_string()).await.ok();
                        });

                        // Spawn writer task
                        let log_tx_writer = log_tx.clone();
                        tokio::spawn(async move {
                            let _ =
                                writer_task(SerialWriter(writer), command_rx, log_tx_writer).await;
                        });
                    }
                    Err(e) => {
                        log_tx
                            .send(format!("Failed to open port: {}", e))
                            .await
                            .ok();
                    }
                }
            }
        });
    });

    // Run the GUI on the main thread
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Serial Plotter"),
        ..Default::default()
    };

    eframe::run_native(
        "Serial Plotter",
        native_options,
        Box::new(move |cc| Ok(Box::new(SerialPlotterApp::new(cc, app_state, connect_tx)))),
    )?;

    Ok(())
}
