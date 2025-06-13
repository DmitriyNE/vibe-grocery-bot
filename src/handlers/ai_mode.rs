use anyhow::{anyhow, Result};
use teloxide::prelude::*;
use tracing::debug;

use crate::ai::detection::{average_closeness, calc_fps, Detector};
use nokhwa::{
    pixel_format::RgbFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
    Camera,
};

pub async fn ai_mode(bot: Bot, msg: Message, model_path: Option<String>) -> Result<()> {
    let path = model_path.unwrap_or_else(|| "yolov8.onnx".to_string());
    let (count, fps) = {
        let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::default());
        let mut camera =
            Camera::new(CameraIndex::Index(0), format).map_err(|e| anyhow!(e.to_string()))?;
        camera.open_stream().map_err(|e| anyhow!(e.to_string()))?;
        let detector = Detector::new(&path)?;
        let frame = camera.frame().map_err(|e| anyhow!(e.to_string()))?;
        let detections = detector.detect(frame.buffer())?;
        let people: Vec<_> = detections
            .iter()
            .filter(|d| d.class == 0)
            .cloned()
            .collect();
        let closeness = average_closeness(&people);
        let fps = calc_fps(people.len(), closeness);
        debug!(people = people.len(), closeness, fps, "ai mode computed");
        (people.len(), fps)
    };

    bot.send_message(msg.chat.id, format!("people: {count}, fps: {fps:.1}"))
        .await?;
    Ok(())
}
