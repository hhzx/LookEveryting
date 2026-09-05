//! Minimal SRT / ASS subtitle parsing and lookup.

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
        for ext in ["srt", "ass", "ssa"] {
            let path = video.with_extension(ext);
            if path.exists() {
                return Self::from_file(&path).ok();
            }
        }
        // Fall back to embedded soft-subtitle tracks (MF).
        let embedded = cap_video::extract_embedded_subtitles(video);
        if embedded.is_empty() {
            return None;
        }
        Some(Self {
            cues: embedded
                .into_iter()
                .map(|c| SubCue {
                    start_secs: c.start_secs,
                    end_secs: c.end_secs,
                    text: c.text,
                })
                .collect(),
        })
    }

    pub fn from_file(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        Ok(match ext.as_str() {
            "ass" | "ssa" => Self::parse_ass(&text),
            _ => Self::parse_srt(&text),
        })
    }

    pub fn from_srt_file(path: &Path) -> Result<Self, String> {
        Self::from_file(path)
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

    /// Minimal ASS/SSA Dialogue parser → plain text cues.
    pub fn parse_ass(input: &str) -> Self {
        let mut cues = Vec::new();
        let mut in_events = false;
        let mut format_cols: Vec<String> = Vec::new();
        for raw in input.lines() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') {
                in_events = line.eq_ignore_ascii_case("[Events]");
                continue;
            }
            if !in_events {
                continue;
            }
            if let Some(rest) = line.strip_prefix("Format:") {
                format_cols = rest
                    .split(',')
                    .map(|s| s.trim().to_ascii_lowercase())
                    .collect();
                continue;
            }
            let Some(rest) = line
                .strip_prefix("Dialogue:")
                .or_else(|| line.strip_prefix("Comment:"))
            else {
                continue;
            };
            if line.starts_with("Comment:") {
                continue;
            }
            let fields: Vec<&str> = split_ass_fields(rest, format_cols.len().max(10));
            let (start_i, end_i, text_i) = if format_cols.is_empty() {
                (1, 2, fields.len().saturating_sub(1))
            } else {
                (
                    format_cols.iter().position(|c| c == "start").unwrap_or(1),
                    format_cols.iter().position(|c| c == "end").unwrap_or(2),
                    format_cols.iter().position(|c| c == "text").unwrap_or(fields.len().saturating_sub(1)),
                )
            };
            let Some(start) = fields.get(start_i).and_then(|s| parse_ass_time(s)) else {
                continue;
            };
            let Some(end) = fields.get(end_i).and_then(|s| parse_ass_time(s)) else {
                continue;
            };
            let text = fields.get(text_i).copied().unwrap_or("");
            let text = strip_ass_tags(text).replace("\\N", "\n").replace("\\n", "\n");
            if text.trim().is_empty() {
                continue;
            }
            cues.push(SubCue {
                start_secs: start,
                end_secs: end,
                text: text.trim().to_string(),
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

fn split_ass_fields(rest: &str, expected: usize) -> Vec<&str> {
    // Text is last and may contain commas.
    let n = expected.max(10);
    rest.splitn(n, ',').map(str::trim).collect()
}

fn parse_ass_time(ts: &str) -> Option<f32> {
    // H:MM:SS.cc
    let ts = ts.trim();
    let mut parts = ts.split(':');
    let h: f32 = parts.next()?.parse().ok()?;
    let m: f32 = parts.next()?.parse().ok()?;
    let s: f32 = parts.next()?.replace(',', ".").parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

fn strip_ass_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            while let Some(n) = chars.next() {
                if n == '}' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_srt_times(line: &str) -> Option<(f32, f32)> {
    let parts: Vec<&str> = line.split("-->").map(str::trim).collect();
    if parts.len() != 2 {
        return None;
    }
    Some((parse_timestamp(parts[0])?, parse_timestamp(parts[1])?))
}

fn parse_timestamp(ts: &str) -> Option<f32> {
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

    #[test]
    fn parses_basic_ass() {
        let ass = "[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:01.00,0:00:02.50,Default,,0,0,0,,{\\i1}Hi{\\i0} there\\NLine2\n";
        let subs = Subtitles::parse_ass(ass);
        assert_eq!(subs.cues.len(), 1);
        assert_eq!(subs.active_at(1.5), Some("Hi there\nLine2"));
    }
}
