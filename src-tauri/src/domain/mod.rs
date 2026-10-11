use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub language: String,
    pub model_id: String,
    pub input_device: Option<String>,
    pub shortcut: String,
    pub shortcut_mode: String,
    pub history_enabled: bool,
    pub retention_days: u32,
    pub overlay_position: String,
    pub autostart: bool,
    pub vad_threshold: f64,
    pub sounds: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "it".into(),
            model_id: "tiny".into(),
            input_device: None,
            shortcut: "CommandOrControl+Shift+Space".into(),
            shortcut_mode: "toggle".into(),
            history_enabled: false,
            retention_days: 30,
            overlay_position: "bottom".into(),
            autostart: false,
            vad_threshold: 0.01,
            sounds: false,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        const WHISPER_LANGUAGES: &str = "af am ar as az ba be bg bn bo br bs ca cs cy da de el en es et eu fa fi fo fr gl gu ha haw he hi hr ht hu hy id is it ja jw ka kk km kn ko la lb ln lo lt lv mg mi mk ml mn mr ms mt my ne nl nn no oc pa pl ps pt ro ru sa sd si sk sl sn so sq sr su sv sw ta te tg th tk tl tr tt uk ur uz vi yi yo yue zh";
        if self.language != "auto"
            && !WHISPER_LANGUAGES
                .split_whitespace()
                .any(|code| code == self.language)
        {
            return Err("language must be 'auto' or a supported Whisper language code".into());
        }
        if self.model_id.trim().is_empty() || self.shortcut.trim().is_empty() {
            return Err("model_id and shortcut must not be empty".into());
        }
        if !matches!(self.shortcut_mode.as_str(), "toggle" | "hold") {
            return Err("shortcut_mode must be 'toggle' or 'hold'".into());
        }
        if !matches!(self.overlay_position.as_str(), "bottom" | "top") {
            return Err("overlay_position must be 'bottom' or 'top'".into());
        }
        if !(1..=3650).contains(&self.retention_days) {
            return Err("retention_days must be between 1 and 3650".into());
        }
        if !self.vad_threshold.is_finite() || !(0.0..=1.0).contains(&self.vad_threshold) {
            return Err("vad_threshold must be finite and between 0 and 1".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub id: String,
    pub started_at_utc: String,
    pub ended_at_utc: Option<String>,
    pub status: String,
    pub recorded_ms: u64,
    pub inference_ms: u64,
    pub insertion_ms: u64,
    pub words_count: u64,
    pub chars_count: u64,
    pub language_code: String,
    pub model_id: String,
    pub error_category: Option<String>,
    pub input_mode: String,
    pub text: Option<String>,
}

impl Session {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("session id must not be empty".into());
        }
        for value in [
            &self.started_at_utc,
            self.ended_at_utc.as_deref().unwrap_or(""),
        ] {
            if !value.is_empty() {
                let parsed = chrono::DateTime::parse_from_rfc3339(value)
                    .map_err(|_| "session timestamps must be RFC 3339")?;
                if parsed.timestamp() < 0 {
                    return Err("session timestamps must be after the Unix epoch".into());
                }
            }
        }
        if !matches!(
            self.status.as_str(),
            "started"
                | "preparing"
                | "recording"
                | "processing"
                | "inserting"
                | "completed"
                | "cancelled"
                | "failed"
                | "error"
        ) {
            return Err("invalid session status".into());
        }
        if [
            self.recorded_ms,
            self.inference_ms,
            self.insertion_ms,
            self.words_count,
            self.chars_count,
        ]
        .iter()
        .any(|value| *value > i64::MAX as u64)
        {
            return Err("session metrics exceed SQLite integer range".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DashboardFilter {
    pub days: u32,
    pub language: Option<String>,
    pub model_id: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub granularity: String,
}

impl Default for DashboardFilter {
    fn default() -> Self {
        Self {
            days: 7,
            language: None,
            model_id: None,
            start_date: None,
            end_date: None,
            granularity: "day".into(),
        }
    }
}

impl DashboardFilter {
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.days, 1 | 7 | 30 | 90) {
            return Err("days must be one of 1, 7, 30, or 90".into());
        }
        if !matches!(self.granularity.as_str(), "day" | "week" | "month") {
            return Err("granularity must be day, week, or month".into());
        }
        let parse_date = |value: &str| {
            chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|_| "dates must use YYYY-MM-DD".to_string())
        };
        let start = self.start_date.as_deref().map(parse_date).transpose()?;
        let end = self.end_date.as_deref().map(parse_date).transpose()?;
        if start.zip(end).is_some_and(|(a, b)| a > b) {
            return Err("start_date must not be after end_date".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Dashboard {
    pub started: u64,
    pub completed: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub words: u64,
    pub chars: u64,
    pub recorded_ms: u64,
    pub inference_ms: u64,
    pub insertion_ms: u64,
    pub mean_inference_ms: f64,
    pub median_inference_ms: f64,
    pub p95_inference_ms: f64,
    pub wpm: f64,
    pub throughput: f64,
    pub active_days: u64,
    pub previous_words: u64,
    pub words_change_pct: Option<f64>,
    pub top_language: Option<String>,
    pub top_model: Option<String>,
    pub timeline: Vec<TimelinePoint>,
    pub heatmap: Vec<HeatmapPoint>,
    pub duration_histogram: Vec<HistogramPoint>,
    pub models: Vec<ModelMetric>,
    pub errors: Vec<ErrorMetric>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TimelinePoint {
    pub date: String,
    pub sessions: u64,
    pub words: u64,
    pub inference_ms: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct HeatmapPoint {
    pub weekday: u32,
    pub hour: u32,
    pub sessions: u64,
    pub words: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct HistogramPoint {
    pub label: String,
    pub count: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ModelMetric {
    pub model_id: String,
    pub sessions: u64,
    pub mean_inference_ms: f64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ErrorMetric {
    pub category: String,
    pub count: u64,
}
