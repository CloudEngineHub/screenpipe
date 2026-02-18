use std::fmt;

#[derive(Clone, Debug, PartialEq, Default)]
pub enum AudioTranscriptionEngine {
    Deepgram,
    WhisperTiny,
    WhisperTinyQuantized,
    #[default]
    WhisperLargeV3Turbo,
    WhisperLargeV3TurboQuantized,
    WhisperLargeV3,
    WhisperLargeV3Quantized,
}

impl std::str::FromStr for AudioTranscriptionEngine {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "deepgram" | "screenpipe-cloud" => Ok(Self::Deepgram),
            "whisper-tiny" => Ok(Self::WhisperTiny),
            "whisper-tiny-quantized" => Ok(Self::WhisperTinyQuantized),
            "whisper-large" => Ok(Self::WhisperLargeV3),
            "whisper-large-quantized" => Ok(Self::WhisperLargeV3Quantized),
            "whisper-large-v3-turbo" => Ok(Self::WhisperLargeV3Turbo),
            "whisper-large-v3-turbo-quantized" => Ok(Self::WhisperLargeV3TurboQuantized),
            _ => Err(format!("unknown audio engine: {s}")),
        }
    }
}

impl fmt::Display for AudioTranscriptionEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioTranscriptionEngine::Deepgram => write!(f, "Deepgram"),
            AudioTranscriptionEngine::WhisperTiny => write!(f, "WhisperTiny"),
            AudioTranscriptionEngine::WhisperTinyQuantized => write!(f, "WhisperTinyQuantized"),
            AudioTranscriptionEngine::WhisperLargeV3 => write!(f, "WhisperLargeV3"),
            AudioTranscriptionEngine::WhisperLargeV3Quantized => {
                write!(f, "WhisperLargeV3Quantized")
            }
            AudioTranscriptionEngine::WhisperLargeV3Turbo => write!(f, "WhisperLargeV3Turbo"),
            AudioTranscriptionEngine::WhisperLargeV3TurboQuantized => {
                write!(f, "WhisperLargeV3TurboQuantized")
            }
        }
    }
}
