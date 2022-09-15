// ---------------- //
// 0. Hacker import //
// ---------------- //
use fundsp::hacker::*;
use vst::buffer::AudioBuffer;
use vst::prelude::*;

struct Synthy {
    // -------------------------------- //
    // 1. Dynamic dispached audio graph //
    // -------------------------------- //
    audio: Box<dyn AudioUnit64 + Send>,
}

impl Plugin for Synthy {
    fn new(_host: HostCallback) -> Self {
        // ------------------------------ //
        // 2. New audio graph description //
        // ------------------------------ //
        let audio_graph = constant(440.0) >> sine() >> split::<U2>();
        Self {
            audio: Box::new(audio_graph) as Box<dyn AudioUnit64 + Send>,
        }
    }
    
    fn get_info(&self) -> Info {
        Info {
            name: "mjg_synthy".into(),
            vendor: "mjg_rusty".into(),
            unique_id: 128956,
            category: Category::Synth,
            inputs: 0,
            outputs: 2,
            parameters: 0,
            ..Info::default()
        }
    }
    
    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        // ------------------------------------------- //
        // 3. Using fundsp to process our audio buffer //
        // ------------------------------------------- //
        //
        // MJG: Does this need to be changed ever?
        //
        let (_, mut outputs) = buffer.split();
        if outputs.len() == 2 {
            let (left, right) = (outputs.get_mut(0), outputs.get_mut(1));
            
            for (left_chunk, right_chunk) in left
            .chunks_mut(MAX_BUFFER_SIZE)
            .zip(right.chunks_mut(MAX_BUFFER_SIZE))
            {
                let mut right_buffer = [0f64; MAX_BUFFER_SIZE];
                let mut left_buffer = [0f64; MAX_BUFFER_SIZE];
                
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
}

vst::plugin_main!(Synthy);
