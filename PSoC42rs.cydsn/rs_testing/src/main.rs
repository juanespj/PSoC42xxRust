mod egui_testing;
mod encoder_tests;
mod serial_plotter;
mod test_tools;
use chrono::Local;
use egui_testing::*;
use serial_plotter::*;
use std::fs::OpenOptions;
use std::io::Write;
use test_tools::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

use tokio_serial::{SerialPortBuilderExt, SerialStream};
// fn main() -> eframe::Result<()> {
//     egui_test();
//     Ok(())
// }

// fn main() -> Result<(), eframe::Error> {
//     let options = eframe::NativeOptions {
//         viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
//         ..Default::default()
//     };

//     eframe::run_native(
//         "Serial Port Data Plotter",
//         options,
//         Box::new(|_cc| Ok(Box::<SerialPlotterApp>::default())),
//     )
// }
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create channels
    let (data_tx, mut data_rx) = mpsc::channel::<DataPoint>(100);
    let (log_tx, mut log_rx) = mpsc::channel::<String>(100);
    let (control_tx, control_rx) = mpsc::channel::<Control>(10);
    let (command_tx, command_rx) = mpsc::channel::<String>(10);

    // Clone for the GUI
    let control_tx_gui = control_tx.clone();
    let command_tx_gui = command_tx.clone();

    // Spawn background tasks
    let app_state = AppState::new(1000);
    let state_clone = app_state.clone();

    // Data receiver task
    tokio::spawn(async move {
        while let Some(dp) = data_rx.recv().await {
            state_clone.add_data_point(dp);
        }
    });

    // Log receiver task
    let state_clone = app_state.clone();
    tokio::spawn(async move {
        while let Some(log) = log_rx.recv().await {
            state_clone.add_log(log);
        }
    });

    // Serial port task (optional - start when user clicks start)
    // You could initialize this when user selects a port
    // For now, this is a placeholder that shows how to connect

    // Uncomment and modify when you have a real serial port:
    /*
    tokio::spawn(async move {
        let port = tokio_serial::new("/dev/ttyUSB0", 9600)
            .open_native_async()
            .expect("Failed to open port");

        let _ = reader_task(port, data_tx, log_tx, control_rx).await;
    });
    */

    // Run the GUI in a separate thread
    std::thread::spawn(move || {
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([800.0, 600.0])
                .with_title("Serial Plotter"),
            ..Default::default()
        };

        let _ = eframe::run_native(
            "Serial Plotter",
            native_options,
            Box::new(|cc| {
                Ok(Box::new(SerialPlotterApp::new(
                    cc,
                    control_tx_gui,
                    command_tx_gui,
                )))
            }),
        );
    });

    // Keep main thread alive
    tokio::signal::ctrl_c().await?;
    Ok(())
}
// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     // Adjust COM port and baud
//     let port = tokio_serial::new("COM6", 230_400).open_native_async()?;

//     serial_read_log_bytes(port, Some("serial_log.txt"), Some(1_000), Some(10_000)).await?;

//     Ok(())
// }

async fn serial_read_log_bytes(
    mut port: SerialStream,
    log_file: Option<&str>,
    max_duration_ms: Option<u64>,
    max_lines: Option<usize>,
) -> anyhow::Result<()> {
    // Send 'r' using AsyncWriteExt explicitly
    AsyncWriteExt::write_all(&mut port, b"s").await?;
    AsyncWriteExt::flush(&mut port).await?;

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

    loop {
        // Apply timeout if requested
        let n = if let Some(timeout_ms) = max_duration_ms {
            match timeout(Duration::from_millis(timeout_ms), port.read(&mut buf)).await {
                Ok(Ok(0)) => continue,
                Ok(Ok(n)) => n,
                Ok(Err(e)) => return Err(e.into()),
                Err(_) => {
                    println!("Timeout reached, sending 'k'");
                    AsyncWriteExt::write_all(&mut port, b"k").await?;
                    AsyncWriteExt::flush(&mut port).await?;
                    break;
                }
            }
        } else {
            let n = port.read(&mut buf).await?;
            if n == 0 {
                continue;
            }
            n
        };

        line_buf.extend_from_slice(&buf[..n]);

        // Split on both newline and comma
        while let Some(pos) = line_buf
            .iter()
            .position(|&b| b == b'\n' || b == b',' || b == b'\r')
        {
            let mut line_bytes: Vec<u8> = line_buf.drain(..=pos).collect();
            // Remove the delimiter (newline or comma)
            line_bytes.pop();
            // Also remove carriage return if present

            if let Ok(line_str) = String::from_utf8(line_bytes) {
                let trimmed = line_str.trim();
                // Skip empty entries
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

                    if lines_read % 10 == 0 {
                        println!("{}", log_line);
                    }

                    lines_read += 1;

                    // Check if we've reached max lines
                    if let Some(max) = max_lines {
                        if lines_read >= max {
                            println!("Max lines ({}) reached, sending 'k'", max);
                            AsyncWriteExt::write_all(&mut port, b"k").await?;
                            AsyncWriteExt::flush(&mut port).await?;
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
