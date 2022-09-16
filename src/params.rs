use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::fmt::Display;
use vst::{plugin::PluginParameters, util::AtomicFloat};

pub struct Parameters {
    pub modulation: AtomicFloat,
    pub counter: AtomicFloat,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            modulation: AtomicFloat::new(1.),
            counter: AtomicFloat::new(0.),
        }
    }
}

impl PluginParameters for Parameters {
    fn get_parameter(&self, index: i32) -> f32 {
        match FromPrimitive::from_i32(index) {
            Some(Parameter::Modulation) => self.modulation.get(),
            Some(Parameter::Counter) => self.counter.get(),

            _ => 0f32,
        }
    }

    #[allow(clippy::single_match)]
    fn set_parameter(&self, index: i32, value: f32) {
        match FromPrimitive::from_i32(index) {
            Some(Parameter::Modulation) => self.modulation.set(value),
            Some(Parameter::Counter) => self.counter.set(value),
            _ => (),
        }
    }

    fn get_parameter_name(&self, index: i32) -> String {
        let param: Option<Parameter> = FromPrimitive::from_i32(index);
        param
            .map(|f| f.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

#[derive(FromPrimitive, Clone, Copy)]
pub enum Parameter {
    Modulation = 0,
    Counter = 1,
}

impl Display for Parameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Parameter::Modulation => "modulation",
                Parameter::Counter => "counter",
            }
        )
    }
}

impl Parameters {
    pub fn modify_parameter(&self, index: i32, f: fn(f32) -> f32) {
        log::info!("Modifying");
        match FromPrimitive::from_i32(index) {
            Some(Parameter::Modulation) => self.modulation.set(f(self.modulation.get())),
            Some(Parameter::Counter) => self.counter.set(f(self.counter.get())),
            _ => (),
        }
    }
}
