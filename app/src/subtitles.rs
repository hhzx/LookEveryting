//! Minimal SRT subtitle parsing and lookup.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct SubCue {
    pub start_secs: f32,
    pub end_secs: f32,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct Subtitles {
    pub cues: Vec<SubCue>,
}

impl Subtitles {
    pub fn load_sidecar(video: &Path) -> Option<Self> {
        let stem = video.with_extension("srt");
        if stem.exists() {
            return Self::from_srt_file(&stem).ok();
        }
        None
    }

    pub fn from_srt_file(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        Ok(Self::parse_srt(&text))
    }

    pub fn parse_srt(input: &str) -> Self {
        let mut cues = Vec::new();
        let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
        for block in normalized.split("\n\n") {
            let lines: Vec<&str> = block
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect();
            if lines.len() < 2 {
                continue;
            }
            let (time_line, text_lines) = if lines[0].chars().all(|c| c.is_ascii_digit()) {
                if lines.len() < 3 {
                    continue;
                }
                (lines[1], &lines[2..])
            } else {
                (lines[0], &lines[1..])
            };
            let Some((start, end)) = parse_srt_times(time_line) else {
                continue;
            };
            let text = text_lines.join("\n");
            if text.is_empty() {
                continue;
            }
            cues.push(SubCue {
                start_secs: start,
                end_secs: end,
                text,
            });
        }
        Self { cues }
    }

    pub fn active_at(&self, secs: f32) -> Option<&str> {
        self.cues
            .iter()
            .find(|c| secs >= c.start_secs && secs <= c.end_secs)
            .map(|c| c.text.as_str())
    }
}

fn parse_srt_times(line: &str) -> Option<(f32, f32)> {
    let parts: Vec<&str> = line.split("-->").map(str::trim).collect();
    if parts.len() != 2 {
        return None;
    }
    Some((parse_timestamp(parts[0])?, parse_timestamp(parts[1])?))
}

fn parse_timestamp(ts: &str) -> Option<f32> {
    // 00:00:01,000 or 00:00:01.000
    let ts = ts.replace(',', ".");
    let parts: Vec<&str> = ts.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: f32 = parts[0].parse().ok()?;
    let m: f32 = parts[1].parse().ok()?;
    let s: f32 = parts[2].parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_srt() {
        let srt = "1\n00:00:01,000 --> 00:00:03,500\nHello\nWorld\n\n2\n00:00:04,000 --> 00:00:05,000\nBye\n";
        let subs = Subtitles::parse_srt(srt);
        assert_eq!(subs.cues.len(), 2);
        assert_eq!(subs.active_at(2.0), Some("Hello\nWorld"));
        assert_eq!(subs.active_at(4.5), Some("Bye"));
        assert!(subs.active_at(10.0).is_none());
    }
}
