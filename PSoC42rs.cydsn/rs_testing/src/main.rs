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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create channels
    let (data_tx, data_rx) = mpsc::channel::<DataPoint>(100);
    let (log_tx, log_rx) = mpsc::channel::<String>(100);
    let (control_tx, control_rx) = mpsc::channel::<Control>(10);
    let (command_tx, command_rx) = mpsc::channel::<String>(10);

    // Clone for the GUI (runs on main thread)
    let control_tx_gui = control_tx.clone();
    let command_tx_gui = command_tx.clone();

    let app_state = AppState::new(1000);

    // Spawn tokio runtime on a background thread
    let state_data = app_state.clone();
    let state_log = app_state.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async {
            let mut data_rx = data_rx;
            tokio::spawn(async move {
                while let Some(dp) = data_rx.recv().await {
                    state_data.add_data_point(dp);
                }
            });

            let mut log_rx = log_rx;
            tokio::spawn(async move {
                while let Some(log) = log_rx.recv().await {
                    state_log.add_log(log);
                }
            });

            // Serial port task placeholder
            /*
            let reader = reader_task(port, data_tx, log_tx, control_rx);
            tokio::spawn(reader);
            */

            // Keep the runtime alive until process exit
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(u64::MAX)).await;
            }
        });
    });

    // Run GUI on the main thread (required by macOS/winit)
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Serial Plotter"),
        ..Default::default()
    };

    eframe::run_native(
        "Serial Plotter",
        native_options,
        Box::new(|cc| {
            Ok(Box::new(SerialPlotterApp::new(
                cc,
                control_tx_gui,
                command_tx_gui,
            )))
        }),
    )
    .map_err(|e| e.into())
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
            match timeout(Duration::from_millis(timeout_ms), AsyncReadExt::read(&mut port, &mut buf)).await {
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
            let n = AsyncReadExt::read(&mut port, &mut buf).await?;
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
