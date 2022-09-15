mod editor;
mod params;

use std::ops::RangeInclusive;

use fundsp::hacker::*;
use num_derive::FromPrimitive;
use params::{Parameter, Parameters};
use std::{convert::TryFrom, sync::Arc, time::Duration};
use vst::prelude::*;
use wmidi::{Note, Velocity};

struct Synthy {
    audio: Box<dyn AudioUnit64 + Send>,
    note: Option<(Note, Velocity)>,
    parameters: Arc<Parameters>,
    // ------------- //
    // 1. New fields //
    // ------------- //
    enabled: bool,
    sample_rate: f32,
    time: Duration,
    editor: Option<editor::PluginEditor>,
}

impl Plugin for Synthy {
    fn init(&mut self) {
        // Set up logs, adapted from code from DGriffin91
        // MIT: https://github.com/DGriffin91/egui_baseview_test_vst2/blob/main/LICENSE
        let Info { name, version, .. } = self.get_info();
        let home = dirs::home_dir().unwrap();
        let id_string = "mjg-synth-log.txt";
        let log_file = std::fs::File::options()
            .append(true)
            .open(home.join(id_string))
            .unwrap();
        let log_config = ::simplelog::ConfigBuilder::new()
            .set_time_to_local(true)
            .build();
        simplelog::WriteLogger::init(simplelog::LevelFilter::Info, log_config, log_file).ok();
        log_panics::init();
        log::info!("{} v{} init", name, version);
    }

    #[allow(clippy::precedence)]
    fn new(_host: HostCallback) -> Self {
        // ------------------------------ //
        // 2. Removal of Parameters::Freq //
        // ------------------------------ //
        let Parameters { modulation } = Parameters::default();

        let freq = || tag(Tag::Freq as i64, 440.);
        let modulation = || tag(Tag::Modulation as i64, modulation.get() as f64);

        // ---------------------- //
        // 3. Envelope generation //
        // ---------------------- //
        let offset = || tag(Tag::NoteOn as i64, 0.);
        let env = || offset() >> envelope2(|t, offset| downarc((t - offset) * 2.));

        let audio_graph = freq()
            >> sine() * freq() * modulation() + freq()
            >> env() * sine()
            >> declick()
            >> split::<U2>();

        let params: Arc<Parameters> = Arc::new(Default::default());
        Self {
            audio: Box::new(audio_graph) as Box<dyn AudioUnit64 + Send>,
            parameters: params.clone(),
            note: None,
            time: Duration::default(),
            sample_rate: 41_000f32,
            enabled: false,
            editor: Some(editor::PluginEditor {
                params,
                window_handle: None,
                is_open: false,
            }),
        }
    }

    // --------------------------- //
    // 4. Changing to 1 parameters //
    // --------------------------- //
    fn get_info(&self) -> Info {
        Info {
            name: "synthy".into(),
            vendor: "rusty".into(),
            unique_id: 128956,
            category: Category::Synth,
            inputs: 0,
            outputs: 2,
            parameters: 1,
            version: 2,
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

                self.set_tag_with_param(Tag::Modulation, Parameter::Modulation, 0f64..=10f64);

                if let Some((note, ..)) = self.note {
                    self.set_tag(Tag::Freq, note.to_freq_f64())
                }

                if self.enabled {
                    // -------------- //
                    // 5. Timekeeping //
                    // -------------- //
                    self.time += Duration::from_secs_f32(MAX_BUFFER_SIZE as f32 / self.sample_rate);
                    self.audio.process(
                        MAX_BUFFER_SIZE,
                        &[],
                        &mut [&mut left_buffer, &mut right_buffer],
                    );
                }

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
                    match midi {
                        wmidi::MidiMessage::NoteOn(_channel, note, velocity) => {
                            // ----------------------------------------- //
                            // 6. Set `NoteOn` time tag and enable synth //
                            // ----------------------------------------- //
                            self.set_tag(Tag::NoteOn, self.time.as_secs_f64());
                            self.note = Some((note, velocity));
                            self.enabled = true;
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

    fn get_editor(&mut self) -> Option<Box<dyn vst::editor::Editor>> {
        if let Some(editor) = self.editor.take() {
            Some(Box::new(editor) as Box<dyn vst::editor::Editor>)
        } else {
            None
        }
    }

    // ------------------------------ //
    // 7. Implement `set_sample_rate` //
    // ------------------------------ //
    fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate;
        self.time = Duration::default();
        self.audio.reset(Some(rate as f64));
    }
}

impl Synthy {
    #[inline(always)]
    fn set_tag(&mut self, tag: Tag, value: f64) {
        self.audio.set(tag as i64, value);
    }

    #[inline(always)]
    fn set_tag_with_param(&mut self, tag: Tag, param: Parameter, range: RangeInclusive<f64>) {
        let value = self.parameters.get_parameter(param as i32) as f64;
        let mapped_value = (value - range.start()) * (range.end() - range.start()) + range.start();
        self.set_tag(tag, mapped_value);
    }
}

#[derive(FromPrimitive, Clone, Copy)]
pub enum Tag {
    Freq = 0,
    Modulation = 1,
    NoteOn = 2,
}

vst::plugin_main!(Synthy);

// fn can_do(&self, can_do: CanDo) -> Supported {
//     match can_do {
//         CanDo::ReceiveEvents => Supported::Yes,
//         CanDo::ReceiveMidiEvent => Supported::Yes,
//         _ => Supported::Maybe,
//     }
// }
