use crate::domain::audio::{KeyDefine, KeyDefineType, MechvibesConfig};
use anyhow::{Context, Result};
use rodio::{Decoder, Source};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct AudioBuffer {
    pub samples: Arc<[i16]>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioBuffer {
    pub fn to_source(&self) -> AudioBufferSource {
        AudioBufferSource {
            buffer: self.samples.clone(),
            pos: 0,
            sample_rate: self.sample_rate,
            channels: self.channels,
            end_pos: self.samples.len(),
        }
    }

    pub fn to_source_slice(&self, start_ms: u64, duration_ms: u64) -> AudioBufferSource {
        let start_frame = (start_ms * self.sample_rate as u64) / 1000;
        let duration_frames = (duration_ms * self.sample_rate as u64) / 1000;

        let start_sample = (start_frame * self.channels as u64) as usize;
        let mut end_sample = start_sample + (duration_frames * self.channels as u64) as usize;

        if end_sample > self.samples.len() {
            end_sample = self.samples.len();
        }

        let pos = if start_sample < self.samples.len() {
            start_sample
        } else {
            self.samples.len()
        };

        AudioBufferSource {
            buffer: self.samples.clone(),
            pos,
            sample_rate: self.sample_rate,
            channels: self.channels,
            end_pos: end_sample,
        }
    }
}

pub struct AudioBufferSource {
    buffer: Arc<[i16]>,
    pos: usize,
    sample_rate: u32,
    channels: u16,
    end_pos: usize,
}

impl Iterator for AudioBufferSource {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.end_pos {
            let sample = self.buffer[self.pos];
            self.pos += 1;
            Some(sample as f32 / 32767.0)
        } else {
            None
        }
    }
}

impl Source for AudioBufferSource {
    fn current_span_len(&self) -> Option<usize> {
        Some((self.end_pos - self.pos) / self.channels as usize)
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let n_frames = (self.end_pos - self.pos) as u64 / self.channels as u64;
        Some(Duration::from_micros(
            n_frames * 1_000_000 / self.sample_rate as u64,
        ))
    }
}

#[derive(Debug)]
pub struct LoadedSoundPack {
    pub config: MechvibesConfig,
    pub buffers: HashMap<String, AudioBuffer>,
}

pub struct SoundPackLoader;

const ALLOWED_SOUND_BASES: &[&str] = &[
    "src/assets/sounds",
    "assets/sounds",
    "/usr/share/hibiki/sounds",
    "/usr/local/share/hibiki/sounds",
];

impl SoundPackLoader {
    fn safe_join(base: &Path, sub: &str) -> Result<PathBuf> {
        let sub_path = Path::new(sub);
        if sub_path.is_absolute()
            || sub_path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            anyhow::bail!("Forbidden path component: {}", sub);
        }
        Ok(base.join(sub_path))
    }

    pub fn get_sound_pack_dir() -> PathBuf {
        for path_str in ALLOWED_SOUND_BASES {
            let path = Path::new(path_str);
            if path.exists() && path.is_dir() {
                return path.to_path_buf();
            }
        }

        PathBuf::from("assets/sounds")
    }

    pub fn list_available_packs() -> Vec<(String, String)> {
        let mut packs = Vec::new();
        let path = Self::get_sound_pack_dir();

        if let Ok(entries) = fs::read_dir(&path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        if let Ok(dir_name) = entry.file_name().into_string() {
                            let config_path = entry.path().join("config.json");
                            let display_name = if let Ok(file) = File::open(&config_path) {
                                let reader = BufReader::new(file);
                                if let Ok(json) =
                                    serde_json::from_reader::<_, serde_json::Value>(reader)
                                {
                                    json.get("name")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| dir_name.clone())
                                } else {
                                    dir_name.clone()
                                }
                            } else {
                                dir_name.clone()
                            };
                            packs.push((dir_name, display_name));
                        }
                    }
                }
            }
        }

        packs.sort_by(|a, b| a.1.cmp(&b.1));
        if packs.is_empty() {
            packs.push(("default".to_string(), "Default".to_string()));
        }
        packs
    }

    pub fn load_from_directory(dir_path: impl AsRef<Path>) -> Result<LoadedSoundPack> {
        let dir_path = dir_path.as_ref();

        if dir_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            anyhow::bail!("Path traversal detected in sound pack path: {:?}", dir_path);
        }

        if dir_path.is_absolute() && !cfg!(test) {
            let mut allowed = false;
            if let Ok(canon_dir) = dir_path.canonicalize() {
                for base in ALLOWED_SOUND_BASES {
                    if let Ok(canon_base) = Path::new(base).canonicalize() {
                        if canon_dir.starts_with(canon_base) {
                            allowed = true;
                            break;
                        }
                    }
                }
            } else if dir_path.file_name().and_then(|n| n.to_str()) == Some("default") {
                if let Some(parent) = dir_path.parent() {
                    if let Ok(canon_parent) = parent.canonicalize() {
                        for base in ALLOWED_SOUND_BASES {
                            if let Ok(canon_base) = Path::new(base).canonicalize() {
                                if canon_parent.starts_with(canon_base) {
                                    allowed = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if !allowed {
                anyhow::bail!("Access to absolute path is restricted: {:?}", dir_path);
            }
        }

        let effective_path = if dir_path.file_name().and_then(|n| n.to_str()) == Some("default")
            && !dir_path.exists()
        {
            let base = dir_path.parent().unwrap_or_else(|| Path::new("."));
            let fallback = base.join("cherrymx-blue-abs");
            if fallback.exists() {
                tracing::info!("Mapping 'default' sound pack to 'cherrymx-blue-abs'");
                fallback
            } else {
                dir_path.to_path_buf()
            }
        } else {
            dir_path.to_path_buf()
        };

        let config_path = effective_path.join("config.json");

        let file = File::open(&config_path)
            .with_context(|| format!("Failed to open config file at {:?}", config_path))?;
        let reader = BufReader::new(file);

        let mut config: MechvibesConfig = serde_json::from_reader(reader)
            .with_context(|| "Failed to parse Mechvibes config.json")?;

        config.path = effective_path;

        Self::load(config)
    }

    pub fn load(config: MechvibesConfig) -> Result<LoadedSoundPack> {
        let mut buffers = HashMap::new();
        let base_path = &config.path;

        match config.key_define_type {
            KeyDefineType::Single => {
                let path = Self::safe_join(base_path, &config.sound)?;
                if path.exists() {
                    match Self::load_file(&path) {
                        Ok(buffer) => {
                            buffers.insert("main".to_string(), buffer);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load single sound file {:?}: {}", path, e)
                        }
                    }
                } else {
                    tracing::warn!("Sound file missing for single pack: {:?}", path);
                }
            }
            KeyDefineType::Multi => {
                for define in config.defines.values() {
                    if let KeyDefine::Multi(Some(filename)) = define {
                        if !buffers.contains_key(filename) {
                            let path = Self::safe_join(base_path, filename)?;
                            if path.exists() {
                                match Self::load_file(&path) {
                                    Ok(buffer) => {
                                        buffers.insert(filename.clone(), buffer);
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to load sound file {:?}: {}",
                                            path,
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(LoadedSoundPack { config, buffers })
    }

    fn load_file(path: &Path) -> Result<AudioBuffer> {
        let file =
            File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| anyhow::anyhow!("Failed to decode audio: {}", e))?;

        let sample_rate = source.sample_rate();
        let channels = source.channels();

        let samples: Vec<i16> = source
            .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect();

        Ok(AudioBuffer {
            samples: samples.into(),
            sample_rate,
            channels,
        })
    }
}
