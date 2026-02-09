mod egui_testing;
mod encoder_tests;
mod test_tools;
use chrono::Local;
use egui_testing::*;
use std::fs::OpenOptions;
use std::io::Write;
use test_tools::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{Duration, timeout};
use tokio_serial::{SerialPortBuilderExt, SerialStream};
#[tokio::main]
async fn main() -> eframe::Result {
    egui_test();
    // Adjust COM port and baud
    // let port = tokio_serial::new("COM3", 115_200).open_native_async()?;

    // serial_read_log_bytes(port, Some("serial_log.txt"), Some(10_000)).await?;

    Ok(())
}

async fn serial_read_log_bytes(
    mut port: SerialStream,
    log_file: Option<&str>,
    max_duration_ms: Option<u64>,
) -> anyhow::Result<()> {
    // Send 'r' using AsyncWriteExt explicitly
    AsyncWriteExt::write_all(&mut port, b"r").await?;
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

        while let Some(pos) = line_buf.iter().position(|&b| b == b'\n') {
            let mut line_bytes: Vec<u8> = line_buf.drain(..=pos).collect();
            if line_bytes.ends_with(&[b'\n']) {
                line_bytes.pop();
            }
            if line_bytes.ends_with(&[b'\r']) {
                line_bytes.pop();
            }

            if let Ok(line_str) = String::from_utf8(line_bytes) {
                let ts = Local::now();
                let log_line = format!(
                    "{}.{:03}: {}",
                    ts.format("%Y-%m-%d %H:%M:%S"),
                    ts.timestamp_subsec_millis(),
                    line_str.trim()
                );

                if let Some(f) = file.as_mut() {
                    writeln!(f, "{}", log_line)?;
                }

                if lines_read % 10 == 0 {
                    println!("{}", log_line);
                }

                lines_read += 1;
            }
        }
    }

    Ok(())
}
