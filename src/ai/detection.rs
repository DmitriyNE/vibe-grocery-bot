use anyhow::Result;
use candle_onnx::onnx::ModelProto;
use prost::Message;
use tracing::{debug, instrument};

#[derive(Debug, Clone, Copy)]
pub struct Detection {
    pub bbox: [f32; 4],
    pub score: f32,
    pub class: i64,
}

pub struct Detector {
    #[allow(dead_code)]
    model: ModelProto,
}

impl Detector {
    #[instrument(level = "debug", skip_all)]
    pub fn new(model_path: &str) -> Result<Self> {
        let data = std::fs::read(model_path)?;
        let model = ModelProto::decode(&*data)?;
        debug!(model_path, "yolo model loaded");
        Ok(Self { model })
    }

    #[instrument(level = "debug", skip(self, _frame))]
    pub fn detect(&self, _frame: &[u8]) -> Result<Vec<Detection>> {
        // TODO: run inference once implemented
        debug!("detect called");
        Ok(Vec::new())
    }
}

pub fn average_closeness(detections: &[Detection]) -> f32 {
    if detections.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0;
    for d in detections {
        let w = (d.bbox[2] - d.bbox[0]).max(0.0);
        let h = (d.bbox[3] - d.bbox[1]).max(0.0);
        sum += w * h;
    }
    sum / detections.len() as f32
}

pub fn calc_fps(num_people: usize, avg_closeness: f32) -> f32 {
    let fps = 60.0 / (1.0 + num_people as f32 + avg_closeness);
    fps.max(5.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_average_closeness_zero() {
        assert_eq!(average_closeness(&[]), 0.0);
    }

    #[test]
    fn test_calc_fps_bounds() {
        let fps = calc_fps(0, 0.0);
        assert!((5.0..=60.0).contains(&fps));
    }

    proptest! {
        #[test]
        fn prop_calc_fps_range(p in 0usize..10, c in 0f32..5.0) {
            let fps = calc_fps(p, c);
            prop_assert!((5.0..=60.0).contains(&fps));
        }
    }
}
