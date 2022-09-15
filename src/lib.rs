mod params;

use fundsp::hacker::*;
use params::{Parameter, Parameters};
use std::{convert::TryFrom, sync::Arc};
use vst::prelude::*;
use wmidi::{Note, Velocity};

const FREQ_SCALAR: f64 = 1000.;

struct Synthy {
    audio: Box<dyn AudioUnit64 + Send>,
    parameters: Arc<Parameters>,
    // -------------------------------- //
    // 1. Storing the note as an option //
    // -------------------------------- //
    note: Option<(Note, Velocity)>,
}

impl Plugin for Synthy {
    #[allow(clippy::precedence)]
    fn new(_host: HostCallback) -> Self {
        let Parameters { freq, modulation } = Parameters::default();
        let hz = freq.get() as f64 * FREQ_SCALAR;

        let freq = || tag(Parameter::Freq as i64, hz);
        let modulation = || tag(Parameter::Modulation as i64, modulation.get() as f64);

        let audio_graph =
            freq() >> sine() * freq() * modulation() + freq() >> sine() >> split::<U2>();

        Self {
            audio: Box::new(audio_graph) as Box<dyn AudioUnit64 + Send>,
            parameters: Default::default(),
            note: None,
        }
    }

    fn get_info(&self) -> Info {
        Info {
            name: "synthy".into(),
            vendor: "rusty".into(),
            unique_id: 128956,
            category: Category::Synth,
            inputs: 0,
            outputs: 2,
            parameters: 2,
            // midi_inputs: 1,   <-- default is 0 which means "all channels"
            ..Info::default()
        }
    }

    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        Arc::clone(&self.parameters) as Arc<dyn PluginParameters>
    }

    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        let (_, mut outputs) = buffer.split();
        if outputs.len() == 2 {
            let (left, right) = (outputs.get_mut(0), outputs.get_mut(1));
            for (left_chunk, right_chunk) in left
                .chunks_mut(MAX_BUFFER_SIZE)
                .zip(right.chunks_mut(MAX_BUFFER_SIZE))
            {
                let mut left_buffer = [0f64; MAX_BUFFER_SIZE];
                let mut right_buffer = [0f64; MAX_BUFFER_SIZE];

                self.audio.set(
                    Parameter::Modulation as i64,
                    self.parameters.get_parameter(Parameter::Modulation as i32) as f64,
                );

                // ------------------------ //
                // 2. Setting the frequency //
                // ------------------------ //
                self.audio.set(
                    Parameter::Freq as i64,
                    self.note.map(|(n, ..)| n.to_freq_f64()).unwrap_or(0.),
                );

                self.audio.process(
                    MAX_BUFFER_SIZE,
                    &[],
                    &mut [&mut left_buffer, &mut right_buffer],
                );

                for (chunk, output) in left_chunk.iter_mut().zip(left_buffer.iter()) {
                    *chunk = *output as f32;
                }

                for (chunk, output) in right_chunk.iter_mut().zip(right_buffer.iter()) {
                    *chunk = *output as f32;
                }
            }
        }
    }

    fn process_events(&mut self, events: &vst::api::Events) {
        for event in events.events() {
            if let vst::event::Event::Midi(midi) = event {
                if let Ok(midi) = wmidi::MidiMessage::try_from(midi.data.as_slice()) {
                    // ------------------------- //
                    // 3. Processing MIDI events //
                    // ------------------------- //
                    match midi {
                        wmidi::MidiMessage::NoteOn(_channel, note, velocity) => {
                            // panic!("oh fucking no");
                            self.note = Some((note, velocity));
                        }
                        wmidi::MidiMessage::NoteOff(_channel, note, _velocity) => {
                            if let Some((current_note, ..)) = self.note {
                                if current_note == note {
                                    self.note = None;
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    fn can_do(&self, can_do: CanDo) -> Supported {
        match can_do {
            CanDo::ReceiveEvents => Supported::Yes,
            CanDo::ReceiveMidiEvent => Supported::Yes,
            _ => Supported::Maybe,
        }
    }
}

vst::plugin_main!(Synthy);
