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
struct EncoderApp {
    vars: Vec<TestVariable>,
    amplitude: f32,
    smooth_ramp: bool,
}

impl Default for EncoderApp {
    fn default() -> Self {
        Self {
            amplitude: 5000.0,
            smooth_ramp: true,
            vars: vec![
                TestVariable::new("Sample Rate us", 10.0, (10.0, 1500.0, 1.0)),
                TestVariable::new("gain A", 0.4, (0.000001, 1.0, 0.0001)),
                TestVariable::new("gain B", 0.1, (0.001, 1.0, 0.001)),
                TestVariable::new("gain C", 0.01, (0.001, 1.0, 0.001)),
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
        egui::SidePanel::left("tuning_knobs").show(ctx, |ui| {
            ui.heading("PSoC4 Parameters");
            for var in self.vars.iter_mut() {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Slider::new(&mut var.value, var.min..=var.max)
                            .step_by(var.step as f64),
                    );
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
            let sim_data = self.run_simulation();
            let plot_h = ui.available_height() / no_plots as f32;
            Plot::new("encoder_plot")
                .legend(Legend::default())
                .height(plot_h)
                .show(ui, |plot_ui| {
                    plot_ui.line(
                        Line::new(sim_data.raw_counts)
                            .name("Raw Encoder (Sampled)")
                            .color(egui::Color32::GRAY),
                    );
                    plot_ui.line(
                        Line::new(sim_data.theta)
                            .name("Filtered Theta")
                            .color(egui::Color32::BLUE),
                    );
                });

            Plot::new("velocity_plot")
                .legend(Legend::default())
                .height(plot_h)
                .show(ui, |plot_ui| {
                    plot_ui.line(
                        Line::new(sim_data.omega)
                            .name("Calculated Omega")
                            .color(egui::Color32::RED),
                    );
                });
            Plot::new("acc_plot")
                .legend(Legend::default())
                .height(plot_h)
                .show(ui, |plot_ui| {
                    plot_ui.line(
                        Line::new(sim_data.alpha)
                            .name("Calculated Alpha")
                            .color(egui::Color32::RED),
                    );
                });
        });
    }
}
struct SimResults {
    raw_counts: PlotPoints,
    theta: PlotPoints,
    omega: PlotPoints,
    alpha: PlotPoints,
}

impl EncoderApp {
    fn run_simulation(&self) -> SimResults {
        let mut test_encoder = Encoder::new(MockEncoder { counter: 0x8000 });
        let mut raw_pts = Vec::new();
        let mut theta_pts = Vec::new();
        let mut omega_pts = Vec::new();
        let mut alpha_pts = Vec::new();

        // config::set_omega_alpha(I32F32::from_num(self.vars[1].value));
        // config::set_omega_eps(I32F32::from_num(self.vars[2].value));
        let sample_time = self.vars[0].value;//us
        config::set_gain_a(I32F32::from_num(self.vars[1].value));
        config::set_gain_b(I32F32::from_num(self.vars[2].value));
        config::set_gain_c(I32F32::from_num(self.vars[3].value));

        // test_encoder.omega_filter = IirFilter::new(I32F32::from_num(self.vars[3].value));
        // config::set_alpha_alpha(I32F32::from_num(self.vars[4].value));
        // config::set_alpha_eps(I32F32::from_num(self.vars[5].value));
        // test_encoder.alpha_filter = IirFilter::new(I32F32::from_num(self.vars[6].value));

        let mut last_sample_time = 0.0;
        let mut current_sampled_value = 0.0;
let dt=I32F32::from_num(sample_time/1_000.0);//ms
        for t in 0..1000 {
            let t_ms=t as f32;
            // 1. Simulate the PSoC4 sampling interval
            if t_ms == 0.0 || t_ms >= last_sample_time + (sample_time/1_000.0)   {
                current_sampled_value =
                    ramp_hold_ramp(t_ms, 200.0, 400.0, 200.0, self.amplitude, self.smooth_ramp);
                last_sample_time = t_ms;
            }

            // 2. Wrap and push to encoder
            let ramp_i32 = current_sampled_value as i32;
            let count = (0x8000 + (ramp_i32 % 1250)) as u32;

            test_encoder.write_enc_counter(count);
            test_encoder.read_counter();

            // 3. Update filter (using the loop dt)
            test_encoder.update(dt  );

            // 4. Record for Plotting
            raw_pts.push([t as f64, current_sampled_value as f64]);
            // theta_pts.push([t as f64, test_encoder.theta.to_num::<f64>()]);
            // omega_pts.push([t as f64, test_encoder.omega.to_num::<f64>()]);
            // alpha_pts.push([t as f64, test_encoder.alpha.to_num::<f64>()]);
            theta_pts.push([t as f64, test_encoder.theta.to_num::<f64>()]);
            omega_pts.push([t as f64, test_encoder.omega.to_num::<f64>()]);
            alpha_pts.push([t as f64, test_encoder.alpha.to_num::<f64>()]);
        }

        SimResults {
            raw_counts: PlotPoints::from(raw_pts),
            theta: PlotPoints::from(theta_pts),
            omega: PlotPoints::from(omega_pts),
            alpha: PlotPoints::from(alpha_pts),
        }
    }
}
