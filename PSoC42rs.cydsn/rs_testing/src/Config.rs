pub struct EncoderConfig {
    pub omega_alpha: f32,
    pub alpha_alpha: f32,
}

impl EncoderConfig {
    pub fn new() -> Self {
        Self {
            omega_alpha: 0.25,
            alpha_alpha: 0.0625,
        }
    }
}
