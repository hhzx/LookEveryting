//! Extract embedded text subtitle tracks via Media Foundation (best-effort).

use std::path::Path;

#[derive(Debug, Clone)]
pub struct EmbeddedCue {
    pub start_secs: f32,
    pub end_secs: f32,
    pub text: String,
}

/// Try to pull soft subtitle cues from the container (SRT/SSA/WebVTT/SAMI tracks).
/// Returns empty when no usable text track is found (common for bitmap PGS/VobSub).
pub fn extract_embedded_subtitles(path: &Path) -> Vec<EmbeddedCue> {
    #[cfg(windows)]
    {
        mf_subs::extract(path).unwrap_or_default()
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Vec::new()
    }
}

#[cfg(windows)]
mod mf_subs {
    use std::path::Path;

    use super::EmbeddedCue;
    use crate::mf_runtime;
    use windows::core::PCWSTR;
    use windows::Win32::Media::MediaFoundation::*;

    pub fn extract(path: &Path) -> Result<Vec<EmbeddedCue>, String> {
        mf_runtime::ensure_initialized();
        unsafe {
            let wide = path_to_file_url(path)?;
            let mut attrs = None;
            MFCreateAttributes(&mut attrs, 1).map_err(|e| e.to_string())?;
            let attrs = attrs.ok_or_else(|| "attrs null".to_string())?;
            let reader = MFCreateSourceReaderFromURL(PCWSTR(wide.as_ptr()), Some(&attrs))
                .map_err(|e| e.to_string())?;

            let mut stream_idx = None;
            let mut subtype = None;
            for i in 0..64u32 {
                let Ok(mt) = reader.GetNativeMediaType(i, 0) else {
                    continue;
                };
                let Ok(major) = mt.GetGUID(&MF_MT_MAJOR_TYPE) else {
                    continue;
                };
                let sub = mt.GetGUID(&MF_MT_SUBTYPE).ok();
                let is_text = major == MFMediaType_Subtitle
                    || major == MFMediaType_SAMI
                    || major == MFMediaType_HTML
                    || (major == MFMediaType_Stream
                        && sub.is_some_and(|s| {
                            s == MFSubtitleFormat_SRT
                                || s == MFSubtitleFormat_SSA
                                || s == MFSubtitleFormat_WebVTT
                                || s == MFSubtitleFormat_TTML
                                || s == MFSubtitleFormat_XML
                        }));
                // Skip known bitmap subtitle formats.
                if sub == Some(MFSubtitleFormat_PGS) || sub == Some(MFSubtitleFormat_VobSub) {
                    continue;
                }
                if is_text {
                    stream_idx = Some(i);
                    subtype = sub;
                    break;
                }
            }
            let Some(stream) = stream_idx else {
                return Ok(Vec::new());
            };

            reader
                .SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false)
                .ok();
            reader
                .SetStreamSelection(stream, true)
                .map_err(|e| e.to_string())?;

            let mut cues = Vec::new();
            let mut guard = 0u32;
            while guard < 10_000 {
                guard += 1;
                let mut flags = 0u32;
                let mut sample = None;
                let mut timestamp = 0i64;
                reader
                    .ReadSample(
                        stream,
                        0,
                        None,
                        Some(&mut flags),
                        Some(&mut timestamp),
                        Some(&mut sample),
                    )
                    .map_err(|e| e.to_string())?;
                if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                    break;
                }
                let Some(sample) = sample else {
                    continue;
                };
                let start = (timestamp.max(0) as f32) / 10_000_000.0;
                let mut duration = 2.0_f32;
                if let Ok(d) = sample.GetSampleDuration() {
                    if d > 0 {
                        duration = (d as f32) / 10_000_000.0;
                    }
                }
                let text = sample_text(&sample)?;
                let text = normalize_subtitle_payload(&text, subtype);
                if text.trim().is_empty() {
                    continue;
                }
                cues.push(EmbeddedCue {
                    start_secs: start,
                    end_secs: start + duration.max(0.2),
                    text,
                });
            }
            Ok(cues)
        }
    }

    fn sample_text(sample: &IMFSample) -> Result<String, String> {
        unsafe {
            let buffer = sample
                .ConvertToContiguousBuffer()
                .map_err(|e| e.to_string())?;
            let mut data: *mut u8 = std::ptr::null_mut();
            let mut cur = 0u32;
            buffer
                .Lock(&mut data, None, Some(&mut cur))
                .map_err(|e| e.to_string())?;
            let bytes = std::slice::from_raw_parts(data, cur as usize);
            let text = if let Ok(s) = std::str::from_utf8(bytes) {
                s.to_string()
            } else {
                // UTF-16 LE BOM or raw wide
                if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
                    let u16s: Vec<u16> = bytes[2..]
                        .chunks_exact(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect();
                    String::from_utf16_lossy(&u16s)
                } else {
                    String::from_utf8_lossy(bytes).to_string()
                }
            };
            buffer.Unlock().ok();
            Ok(text)
        }
    }

    fn normalize_subtitle_payload(raw: &str, subtype: Option<windows_core::GUID>) -> String {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        // WebVTT payload may include cue header lines.
        if subtype == Some(MFSubtitleFormat_WebVTT) || trimmed.starts_with("WEBVTT") {
            return strip_webvtt(trimmed);
        }
        if subtype == Some(MFSubtitleFormat_SSA) || trimmed.contains("Dialogue:") {
            // Single ASS dialogue line or fragment — take text after last commas block.
            if let Some(pos) = trimmed.rfind(",,") {
                return strip_ass_tags(&trimmed[pos + 2..]);
            }
        }
        strip_ass_tags(trimmed)
            .replace("\\N", "\n")
            .replace("\\n", "\n")
            .trim()
            .to_string()
    }

    fn strip_webvtt(input: &str) -> String {
        let mut lines = Vec::new();
        for line in input.lines() {
            let l = line.trim();
            if l.is_empty() || l.starts_with("WEBVTT") || l.contains("-->") || l.chars().all(|c| c.is_ascii_digit())
            {
                continue;
            }
            if l.starts_with("NOTE") || l.starts_with("STYLE") {
                continue;
            }
            lines.push(l);
        }
        lines.join("\n")
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

    fn path_to_file_url(path: &Path) -> Result<Vec<u16>, String> {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let mut text = path.to_string_lossy().to_string();
        if let Some(stripped) = text.strip_prefix(r"\\?\") {
            text = stripped.to_string();
        }
        let url = if text.starts_with("//") {
            format!("file:{text}")
        } else {
            format!("file:///{}", text.replace('\\', "/"))
        };
        Ok(url.encode_utf16().chain(std::iter::once(0)).collect())
    }
}
