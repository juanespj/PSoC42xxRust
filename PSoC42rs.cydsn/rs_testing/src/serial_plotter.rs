use chrono::Local;
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{Duration, timeout};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

const MAX_PLOT_POINTS: usize = 1000;

#[derive(Clone)]
struct DataPoint {
    timestamp: f64,
    value: f64,
}

struct SerialPlotterApp {
    // Serial port state
    port_path: String,
    baud_rate: String,
    is_connected: bool,
    is_reading: bool,

    // Data storage
    data_points: Arc<Mutex<VecDeque<DataPoint>>>,

    // UI state
    log_text: Arc<Mutex<String>>,
    max_lines: String,
    log_file: String,
    enable_logging: bool,

    // Runtime handle for async operations
    runtime: Arc<tokio::runtime::Runtime>,

    // Control handles - separate ports for reading and writing
    read_port: Arc<Mutex<Option<SerialStream>>>,
    write_port: Arc<Mutex<Option<SerialStream>>>,
}

impl Default for SerialPlotterApp {
    fn default() -> Self {
        Self {
            port_path: "/dev/ttyUSB0".to_string(),
            baud_rate: "115200".to_string(),
            is_connected: false,
            is_reading: false,
            data_points: Arc::new(Mutex::new(VecDeque::new())),
            log_text: Arc::new(Mutex::new(String::new())),
            max_lines: "1000".to_string(),
            log_file: "serial_log.txt".to_string(),
            enable_logging: false,
            runtime: Arc::new(tokio::runtime::Runtime::new().unwrap()),
            read_port: Arc::new(Mutex::new(None)),
            write_port: Arc::new(Mutex::new(None)),
        }
    }
}

impl SerialPlotterApp {
    fn connect_serial(&mut self) {
        let port_path = self.port_path.clone();
        let baud_rate: u32 = self.baud_rate.parse().unwrap_or(115200);

        // Open two ports - one for reading, one for writing
        // match tokio_serial::new(&port_path, baud_rate).open() {
        //     Ok(read_port) => match tokio_serial::new(&port_path, baud_rate).open_native_async() {
        //         Ok(write_port) => {
        //             *self.read_port.lock().unwrap() = Some(read_port);
        //             *self.write_port.lock().unwrap() = Some(write_port);
        //             self.is_connected = true;
        //             self.append_log(&format!("Connected to {} at {} baud", port_path, baud_rate));
        //         }
        //         Err(e) => {
        //             self.append_log(&format!("Failed to open write port: {}", e));
        //         }
        //     },
        //     Err(e) => {
        //         self.append_log(&format!("Failed to open read port: {}", e));
        //     }
        // }
    }

    fn disconnect_serial(&mut self) {
        *self.read_port.lock().unwrap() = None;
        *self.write_port.lock().unwrap() = None;
        self.is_connected = false;
        self.is_reading = false;
        self.append_log("Disconnected");
    }

    fn send_command(&self, cmd: &[u8]) {
        let write_port = self.write_port.clone();
        let runtime = self.runtime.clone();
        let log_text = self.log_text.clone();
        let cmd_vec = cmd.to_vec();

        // runtime.spawn(async move {
        //     if let Some(mut port) = write_port
        //         .lock()
        //         .unwrap()
        //         .as_ref()
        //         .map(|p| p.try_clone().unwrap())
        //     {
        //         match AsyncWriteExt::write_all(&mut port, &cmd_vec).await {
        //             Ok(_) => {
        //                 let _ = AsyncWriteExt::flush(&mut port).await;
        //                 let msg = format!("Sent command: {:?}", String::from_utf8_lossy(&cmd_vec));
        //                 log_text.lock().unwrap().push_str(&format!("{}\n", msg));
        //             }
        //             Err(e) => {
        //                 let msg = format!("Failed to send command: {}", e);
        //                 log_text.lock().unwrap().push_str(&format!("{}\n", msg));
        //             }
        //         }
        //     }
        // });
    }

    fn start_reading(&mut self) {
        if !self.is_connected {
            self.append_log("Not connected to serial port");
            return;
        }

        // Take ownership of the read port for the reading task
        let port = match self.read_port.lock().unwrap().take() {
            Some(p) => p,
            None => {
                self.append_log("Read port not available");
                return;
            }
        };

        let read_port = self.read_port.clone();
        let data_points = self.data_points.clone();
        let log_text = self.log_text.clone();
        let runtime = self.runtime.clone();
        let max_lines: usize = self.max_lines.parse().unwrap_or(1000);
        let log_file = if self.enable_logging {
            Some(self.log_file.clone())
        } else {
            None
        };

        self.is_reading = true;
        self.append_log("Started reading data");

        runtime.spawn(async move {
            let result = serial_read_and_plot(
                port,
                log_file.as_deref(),
                Some(max_lines),
                data_points,
                log_text.clone(),
            )
            .await;

            // Note: port is consumed by serial_read_and_plot, so we can't put it back
            // To restart reading, you'll need to reconnect

            if let Err(e) = result {
                log_text
                    .lock()
                    .unwrap()
                    .push_str(&format!("Error reading serial: {}\n", e));
            }

            log_text.lock().unwrap().push_str("Reading stopped\n");
        });
    }

    fn stop_reading(&mut self) {
        self.is_reading = false;
        self.send_command(b"k");
        self.append_log("Stopped reading data");
    }

    fn append_log(&self, msg: &str) {
        let ts = Local::now();
        let log_line = format!(
            "{}.{:03}: {}",
            ts.format("%Y-%m-%d %H:%M:%S"),
            ts.timestamp_subsec_millis(),
            msg
        );
        self.log_text
            .lock()
            .unwrap()
            .push_str(&format!("{}\n", log_line));
    }

    fn clear_data(&mut self) {
        self.data_points.lock().unwrap().clear();
        self.append_log("Cleared plot data");
    }
}

impl eframe::App for SerialPlotterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request continuous repaint for real-time updates
        ctx.request_repaint();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading("Serial Port Data Plotter");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Port:");
                ui.text_edit_singleline(&mut self.port_path);

                ui.label("Baud:");
                ui.text_edit_singleline(&mut self.baud_rate);

                if self.is_connected {
                    if ui.button("Disconnect").clicked() {
                        self.disconnect_serial();
                    }
                } else {
                    if ui.button("Connect").clicked() {
                        self.connect_serial();
                    }
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Send 's' (Start)").clicked() && self.is_connected {
                    self.send_command(b"s");
                    if !self.is_reading {
                        self.start_reading();
                    }
                }

                if ui.button("Send 'k' (Stop)").clicked() && self.is_connected {
                    self.stop_reading();
                }

                if ui.button("Clear Data").clicked() {
                    self.clear_data();
                }

                ui.separator();

                ui.label("Max Lines:");
                ui.text_edit_singleline(&mut self.max_lines);

                ui.checkbox(&mut self.enable_logging, "Log to file");
                if self.enable_logging {
                    ui.label("File:");
                    ui.text_edit_singleline(&mut self.log_file);
                }
            });
        });

        egui::SidePanel::right("log_panel")
            .min_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Log");
                ui.separator();

                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        let log = self.log_text.lock().unwrap();
                        ui.label(log.as_str());
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Data Plot");

            let data_points = self.data_points.lock().unwrap();

            if !data_points.is_empty() {
                let points: PlotPoints = data_points
                    .iter()
                    .map(|dp| [dp.timestamp, dp.value])
                    .collect();

                let line = Line::new(points);

                Plot::new("serial_plot")
                    .view_aspect(2.0)
                    .show(ui, |plot_ui| {
                        plot_ui.line(line);
                    });

                ui.label(format!("Points: {}", data_points.len()));
            } else {
                ui.label("No data to display. Send 's' to start reading.");
            }
        });
    }
}

async fn serial_read_and_plot(
    mut port: SerialStream,
    log_file: Option<&str>,
    max_lines: Option<usize>,
    data_points: Arc<Mutex<VecDeque<DataPoint>>>,
    log_text: Arc<Mutex<String>>,
) -> anyhow::Result<()> {
    let mut buf = [0u8; 64];
    let mut line_buf = Vec::new();
    let mut file = if let Some(filename) = log_file {
        Some(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(filename)?,
        )
    } else {
        None
    };
    let mut lines_read = 0;
    let start_time = std::time::Instant::now();

    loop {
        let n = match timeout(Duration::from_millis(100), port.read(&mut buf)).await {
            Ok(Ok(0)) => continue,
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => continue, // Timeout, continue loop
        };

        line_buf.extend_from_slice(&buf[..n]);

        while let Some(pos) = line_buf
            .iter()
            .position(|&b| b == b'\n' || b == b',' || b == b'\r')
        {
            let mut line_bytes: Vec<u8> = line_buf.drain(..=pos).collect();
            line_bytes.pop(); // Remove delimiter

            if let Ok(line_str) = String::from_utf8(line_bytes) {
                let trimmed = line_str.trim();
                if !trimmed.is_empty() {
                    let ts = Local::now();
                    let log_line = format!(
                        "{}.{:03}: {}",
                        ts.format("%Y-%m-%d %H:%M:%S"),
                        ts.timestamp_subsec_millis(),
                        trimmed
                    );

                    if let Some(f) = file.as_mut() {
                        writeln!(f, "{}", log_line)?;
                    }

                    // Try to parse as number for plotting
                    if let Ok(value) = trimmed.parse::<f64>() {
                        let elapsed = start_time.elapsed().as_secs_f64();
                        let mut points = data_points.lock().unwrap();
                        points.push_back(DataPoint {
                            timestamp: elapsed,
                            value,
                        });

                        // Keep only last MAX_PLOT_POINTS
                        while points.len() > MAX_PLOT_POINTS {
                            points.pop_front();
                        }
                    }

                    if lines_read % 10 == 0 {
                        log_text
                            .lock()
                            .unwrap()
                            .push_str(&format!("{}\n", log_line));
                    }

                    lines_read += 1;

                    if let Some(max) = max_lines {
                        if lines_read >= max {
                            let msg = format!("Max lines ({}) reached, stopping", max);
                            log_text.lock().unwrap().push_str(&format!("{}\n", msg));
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}
