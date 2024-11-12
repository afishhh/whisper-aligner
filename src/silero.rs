use std::{path::Path, sync::Arc};

use ndarray::{Array, ArrayView, ArrayView2, Dim};
use ort::{execution_providers::CPUExecutionProviderOptions, Environment, ExecutionProvider};

const STATE_DIM: [usize; 3] = [2, 1, 128];

pub struct Silero {
    session: ort::Session,
    sample_rate: i64,
    state: Array<f32, Dim<[usize; 3]>>,
}

pub enum SampleRate {
    #[allow(dead_code)]
    Hz8000,
    Hz16000,
}

impl Silero {
    pub fn new(sample_rate: SampleRate, model: impl AsRef<Path>) -> ort::OrtResult<Self> {
        Ok(Silero {
            session: ort::SessionBuilder::new(&Arc::new(Environment::builder().build()?))?
                .with_execution_providers(&[ExecutionProvider::CPU(
                    CPUExecutionProviderOptions::default(),
                )])?
                .with_model_from_file(model)?,
            sample_rate: match sample_rate {
                SampleRate::Hz8000 => 8000,
                SampleRate::Hz16000 => 16000,
            },
            state: Array::zeros(STATE_DIM),
        })
    }

    pub fn run(&mut self, audio_frame: &[f32]) -> ort::OrtResult<f32> {
        let mut frame = ArrayView2::from_shape([1, audio_frame.len()], audio_frame).unwrap();

        frame = frame.slice_move(ndarray::s![.., ..480]);
        let sample_rate_array = unsafe { ArrayView::from_shape_ptr([1], &self.sample_rate) };
        let frame_view = frame.view().into_dyn().into();
        let state_view = self.state.view().into_dyn().into();
        let sample_rate_view = sample_rate_array.view().into_dyn().into();
        let result = self.session.run(vec![
            ort::Value::from_array(self.session.allocator(), &frame_view)?,
            ort::Value::from_array(self.session.allocator(), &state_view)?,
            ort::Value::from_array(self.session.allocator(), &sample_rate_view)?,
        ])?;
        let [output, new_state] = result.as_slice() else {
            panic!(
                "silero onnx result contains unexpected number of values {}",
                result.len()
            )
        };
        self.state = new_state
            .try_extract()?
            .view()
            .clone()
            .into_dimensionality()
            .unwrap()
            .to_owned();
        Ok(*output.try_extract()?.view().to_owned().first().unwrap())
    }
}
