use crate::test_tools::*;
use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use rust_core::encoder_core::*;
use rust_core::utils_core::*;

use fixed::types::I32F32;

pub fn egui_test() -> eframe::Result {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Encoder Filter Tuner",
        options,
        Box::new(|_cc| Ok(Box::<EncoderApp>::default())),
    )
}
#[derive(Clone)]

struct TestVariable {
    pub name: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
}

impl TestVariable {
    fn new(name: &str, val: f32, range: (f32, f32, f32)) -> Self {
        Self {
            name: name.to_string(),
            value: val,
            min: range.0,
            max: range.1,
            step: range.2,
        }
    }
}
#[derive(Clone)]
struct EncoderApp {
    vars: Vec<TestVariable>,
    first_run: bool,
    amplitude: f32,
    smooth_ramp: bool,
    sim_data: SimResults,
}
const TICK_PER_US: f32 = 1.0 / 24.0; // 24 MHz clock =  1 tick = 0.0416 us (24 MHz clock)
impl Default for EncoderApp {
    fn default() -> Self {
        Self {
            first_run: true,
            amplitude: 5000.0,
            smooth_ramp: true,
            sim_data: SimResults::default(),
            vars: vec![
                //220 us sample time
                TestVariable::new("Sample Rate us", 1200.0, (150.0, 800.0, 1.0)),
                TestVariable::new("gain A", 0.058, (0.0001, 1.0, 0.0001)),
                TestVariable::new("gain B", 0.01, (0.0001, 0.5, 0.0001)),
                TestVariable::new("gain C", 0.05, (0.0001, 0.1, 0.0001)),
                // TestVariable::new("Sample Rate", 1.0, (0.5, 3.0, 0.10)),
                // TestVariable::new("omega_alpha", 0.5, (0.5, 1.0, 0.01)),
                // TestVariable::new("omega_eps", 0.8, (0.001, 10.0, 0.1)),
                // TestVariable::new("omega_filter", 0.6, (0.001, 1.0, 0.005)),
                // TestVariable::new("alpha_alpha", 0.1, (0.0001, 1.0, 0.0001)),
                // TestVariable::new("alpha_eps", 0.5, (0.0001, 100.0, 0.1)),
                // TestVariable::new("alpha_filter", 0.18, (0.0001, 1.0, 0.0001)),
            ],
        }
    }
}

impl eframe::App for EncoderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut inputchanged = false;

        egui::SidePanel::left("tuning_knobs").show(ctx, |ui| {
            ui.heading("PSoC4 Parameters");

            for var in self.vars.iter_mut() {
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Slider::new(&mut var.value, var.min..=var.max)
                                .step_by(var.step as f64),
                        )
                        .changed()
                    {
                        inputchanged = true;
                    }
                    ui.label(&var.name);
                    if ui.button("📋").on_hover_text("Copy to clipboard").clicked() {
                        ui.output_mut(|o| {
                            o.copied_text =
                                format!("{:#010X}", I32F32::from_num(var.value).to_bits())
                        });
                    }
                });
            }
        });
        let no_plots = 3;
        egui::CentralPanel::default().show(ctx, |ui| {
            if inputchanged || self.first_run {
                self.run_simulation();
                self.first_run = false;
            }

            let plot_h = ui.available_height() / no_plots as f32;
            Plot::new("encoder_plot")
                .legend(Legend::default())
                .height(plot_h)
                .show(ui, |plot_ui| {
                    let plot_points = PlotPoints::from_ys_f64(&self.sim_data.raw_counts);
                    plot_ui.line(
                        Line::new(plot_points)
                            .name("Raw Encoder (Sampled)")
                            .color(egui::Color32::GRAY),
                    );
                    let plot_points = PlotPoints::from_ys_f64(&self.sim_data.theta);

                    plot_ui.line(
                        Line::new(plot_points)
                            .name("Filtered Theta")
                            .color(egui::Color32::BLUE),
                    );
                });

            Plot::new("velocity_plot")
                .legend(Legend::default())
                .height(plot_h)
                .show(ui, |plot_ui| {
                    let plot_points = PlotPoints::from_ys_f64(&self.sim_data.omega);

                    plot_ui.line(
                        Line::new(plot_points)
                            .name("Calculated Omega")
                            .color(egui::Color32::RED),
                    );
                });
            Plot::new("acc_plot")
                .legend(Legend::default())
                .height(plot_h)
                .show(ui, |plot_ui| {
                    let plot_points = PlotPoints::from_ys_f64(&self.sim_data.alpha);

                    plot_ui.line(
                        Line::new(plot_points)
                            .name("Calculated Alpha")
                            .color(egui::Color32::RED),
                    );
                });
        });
    }
}
#[derive(Clone)]
struct SimResults {
    t: Vec<f64>,
    raw_counts: Vec<f64>,
    theta: Vec<f64>,
    omega: Vec<f64>,
    alpha: Vec<f64>,
}
impl Default for SimResults {
    fn default() -> Self {
        Self {
            t: vec![],
            raw_counts: vec![],
            theta: vec![],
            omega: vec![],
            alpha: vec![],
        }
    }
}
use rust_core::encoder_core::SCALE;
impl EncoderApp {
    fn run_simulation(&mut self) {
        let mut test_encoder = Encoder::new(MockEncoder { counter: 0x8000 });
        self.sim_data.t.clear();
        self.sim_data.raw_counts.clear();
        self.sim_data.theta.clear();
        self.sim_data.omega.clear();
        self.sim_data.alpha.clear();

        // --- Configuration ---

        let sample_time_us = self.vars[0].value;

        config::set_gain_a((self.vars[1].value * SCALE as f32) as u64);
        config::set_gain_b((self.vars[2].value * SCALE as f32) as u64);
        config::set_gain_c((self.vars[3].value * SCALE as f32) as u64);

        // --- Simulation state ---
        let mut sim_time_us = 0.0;
        let mut current_sampled_value: f32;
        let mut printres = 0;
        while sim_time_us < 900_000.0 {
            sim_time_us += sample_time_us;
            let t_ms = sim_time_us * 0.001;

            // 1. Generate signal
            current_sampled_value =
                ramp_hold_ramp(t_ms, 200.0, 400.0, 200.0, self.amplitude, self.smooth_ramp);

            // 2. Wrap into encoder counts
            let ramp_i32 = current_sampled_value as i32;
            let count = (0x8000 + (ramp_i32 % 1250)) as u32;

            test_encoder.write_enc_counter(count);
            test_encoder.read_counter();

            // 3. Update filter (tick-accurate)
            test_encoder.update();

            // 4. Record
            if printres > 30 {
                printres = 0;

                self.sim_data.t.push(t_ms as f64);
                self.sim_data.raw_counts.push(current_sampled_value as f64);
                self.sim_data.theta.push(test_encoder.theta as f64);
                self.sim_data.omega.push(test_encoder.omega as f64);
                self.sim_data.alpha.push(test_encoder.alpha as f64);
            }
            printres += 1;
        }
    }
}
