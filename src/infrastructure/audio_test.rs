#[cfg(test)]
mod tests {
    use crate::infrastructure::audio::{AudioBuffer, SoundPackLoader};
    use hound;
    use std::fs::File;
    use std::io::Write;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn test_load_mechvibes_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        let sound_path = dir.path().join("sound.wav");

        let config_content = r#"{
            "id": "test",
            "name": "Test Pack",
            "key_define_type": "single",
            "includes_numpad": false,
            "sound": "sound.wav",
            "defines": {
                "1": [0, 100]
            }
        }"#;

        let mut file = File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&sound_path, spec).unwrap();
        for _ in 0..100 {
            writer.write_sample(0i16).unwrap();
        }
        writer.finalize().unwrap();

        let pack = SoundPackLoader::load_from_directory(dir.path());

        let pack = pack.expect("Failed to load pack");

        assert_eq!(pack.config.id, "test");
        assert!(pack.buffers.get("main").is_some());

        let buffer = pack.buffers.get("main").unwrap();
        assert_eq!(buffer.samples.len(), 100);
    }

    #[test]
    fn test_audio_buffer_conversion() {
        let samples: Arc<[i16]> = Arc::new([0, 16383, 32767, -16384]);
        let buffer = AudioBuffer {
            samples: samples.clone(),
            sample_rate: 44100,
            channels: 1,
        };

        let source = buffer.to_source();
        let collected: Vec<f32> = source.collect();

        assert_eq!(collected.len(), 4);
        assert!((collected[0] - 0.0).abs() < 0.001);
        assert!((collected[1] - 0.5).abs() < 0.001);
        assert!((collected[2] - 1.0).abs() < 0.001);
        assert!((collected[3] + 0.5).abs() < 0.001);
    }
}
