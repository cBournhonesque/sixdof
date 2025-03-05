use std::time::Duration;

use kira::{effect::eq_filter::EqFilterKind, Decibels, Easing, Mapping, Value};

#[derive(Debug, Clone)]
pub struct ReverbSettings {
    pub damping: f64,
    pub feedback: f64,
    pub mix: kira::Mix,
    pub volume: Value<Decibels>,
}

impl Default for ReverbSettings {
    fn default() -> Self {
        Self {
            damping: 0.5,
            feedback: 0.5,
            mix: kira::Mix::WET,
            volume: Value::FromListenerDistance(Mapping {
                input_range: (0.0, 100.0),
                output_range: (Decibels(6.0), Decibels(40.0)),
                easing: Easing::Linear,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LowPassSettings {
    pub cutoff_hz: Value<f64>,
}

impl Default for LowPassSettings {
    fn default() -> Self {
        Self {
            cutoff_hz: Value::FromListenerDistance(Mapping {
                input_range: (1.0, 50.0),
                output_range: (20000.0, 500.0),
                easing: Easing::Linear,
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct EqFrequency {
    pub kind: EqFilterKind,
    pub frequency: f64,
    pub gain: Value<Decibels>,
    pub q: f64,
}

#[derive(Debug, Clone)]
pub struct EqSettings {
    pub frequencies: Vec<EqFrequency>,
}

impl Default for EqSettings {
    fn default() -> Self {
        Self { 
            frequencies: vec![
                EqFrequency { kind: EqFilterKind::Bell, frequency: 100.0, gain: Value::Fixed(Decibels(0.0)), q: 1.0 },
                EqFrequency { kind: EqFilterKind::Bell, frequency: 1000.0, gain: Value::Fixed(Decibels(0.0)), q: 1.0 },
                EqFrequency { kind: EqFilterKind::Bell, frequency: 10000.0, gain: Value::Fixed(Decibels(0.0)), q: 1.0 },
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct DelaySettings {
    pub delay_time: Duration,
    pub feedback: Value<Decibels>,
}

impl Default for DelaySettings {
    fn default() -> Self {
        Self { delay_time: Duration::from_secs(1), feedback: Value::Fixed(Decibels(0.0)) }
    }
}
