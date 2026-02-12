# Serial Port Data Plotter

A real-time serial port data plotter built with egui and Rust. This application reads numeric data from a serial port and displays it in an interactive plot.

## Features

- **Real-time plotting**: Visualizes numeric data as it arrives from the serial port
- **Serial control**: Send 's' to start and 'k' to stop data collection
- **Configurable serial port**: Easily set port path and baud rate
- **Data logging**: Optional file logging with timestamps
- **Interactive UI**: 
  - Connect/disconnect to serial ports
  - Clear plot data
  - Scrolling log panel
  - Configurable max lines

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run --release
```

## Usage

1. **Configure Serial Port**:
   - Enter your serial port path (e.g., `/dev/ttyUSB0` on Linux, `COM3` on Windows)
   - Set baud rate (default: 115200)

2. **Connect**:
   - Click "Connect" to establish connection

3. **Start Data Collection**:
   - Click "Send 's' (Start)" to begin reading data
   - The application automatically sends 's' and starts the reading loop

4. **Stop Data Collection**:
   - Click "Send 'k' (Stop)" to stop reading data

5. **View Data**:
   - The plot displays numeric values over time
   - Log panel shows timestamped entries
   - Non-numeric data is logged but not plotted

6. **Optional Features**:
   - Enable "Log to file" to save all data to a file
   - Set "Max Lines" to automatically stop after a certain number of readings
   - Click "Clear Data" to reset the plot

## Data Format

The application expects data from the serial port in one of these formats:
- Newline-delimited values: `123.45\n`
- Comma-delimited values: `123.45,456.78,`
- Carriage return delimited: `123.45\r\n`

Each value that can be parsed as a number (f64) will be plotted. Non-numeric data will be logged but not plotted.

## Architecture

- **egui**: Provides the GUI framework
- **egui_plot**: Handles the plotting functionality
- **tokio**: Async runtime for non-blocking serial I/O
- **tokio-serial**: Async serial port communication
- **Arc<Mutex<>>**: Thread-safe data sharing between async tasks and UI

## Limitations

- Maximum of 1000 points displayed on plot (oldest points are removed)
- Serial data must be text-based (UTF-8)
- Plotting only works with numeric values
