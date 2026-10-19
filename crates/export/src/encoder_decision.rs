//! ours (self-logging): one `decision: export:` line per export naming the codec, the encoder that
//! actually took it and why (`Workspaces/_dev/SELF-LOGGING.md`, contract C). Measured 2026-10-18 on
//! an M1: an H.264 export ran 2.5x faster with Hardware Encoding on than off and the log said neither.

use crate::{Format, HardwareEncoding};

/// Why `encoder` took a `format` export, given the Hardware Encoding setting and whether a hardware
/// encoder declined the configuration while the encoder was being chosen.
pub fn reason(format: Format, encoder: &str, hw: HardwareEncoding, declined: bool) -> &'static str {
    let built_in = encoder == crate::BUILT_IN_ENCODER;
    match (format, built_in) {
        (Format::Hevc, false) => "H.265 is encoded in hardware only",
        (_, false) => "Hardware Encoding is on",
        (Format::H264, true) if hw != HardwareEncoding::Auto => "Hardware Encoding is off",
        (Format::H264, true) if declined => "fallback: the hardware encoder declined this configuration (the line before says why)",
        (Format::H264, true) => "no hardware H.264 encoder on this machine",
        (_, true) => "the only encoder for this format",
    }
}

/// The line itself.
pub fn line(format: Format, encoder: &str, hw: HardwareEncoding, declined: bool) -> String {
    format!("decision: export: {} encoded by {encoder} ({})", format.label(), reason(format, encoder, hw, declined))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExportSettings;

    #[test]
    fn the_reason_names_the_setting_or_the_fallback() {
        let b = crate::BUILT_IN_ENCODER;
        assert_eq!(reason(Format::H264, "VideoToolbox", HardwareEncoding::Auto, false), "Hardware Encoding is on");
        assert_eq!(reason(Format::Hevc, "VideoToolbox", HardwareEncoding::Off, false), "H.265 is encoded in hardware only");
        assert_eq!(reason(Format::H264, b, HardwareEncoding::Off, false), "Hardware Encoding is off");
        assert!(reason(Format::H264, b, HardwareEncoding::Auto, true).starts_with("fallback:"));
        assert_eq!(reason(Format::H264, b, HardwareEncoding::Auto, false), "no hardware H.264 encoder on this machine");
        assert_eq!(reason(Format::ProRes, b, HardwareEncoding::Auto, false), "the only encoder for this format");
    }

    #[test]
    fn choosing_an_encoder_writes_one_decision_line() {
        let _log = craft_log::testing::capture();
        let s = ExportSettings { format: Format::H264, hardware_encoding: HardwareEncoding::Off, ..Default::default() };
        let enc = crate::job::make_venc(&s, 64, 48, filmcraft_time::FrameRate::new(30, 1)).expect("the built-in H.264 encoder");
        assert_eq!(enc.encoder_name(), crate::BUILT_IN_ENCODER);
        let lines: Vec<String> = craft_log::testing::captured().into_iter().filter(|l| l.contains("decision: export:")).collect();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("H.264 encoded by built-in (software) (Hardware Encoding is off)"), "{}", lines[0]);
    }

    #[test]
    fn auto_with_no_hardware_factory_says_there_is_none() {
        // Legion Go, 2026-10-18: Auto on a machine with no hardware encoder said "declined".
        // This crate registers no hardware factory, as on a machine without one.
        let _log = craft_log::testing::capture();
        let s = ExportSettings { format: Format::H264, hardware_encoding: HardwareEncoding::Auto, ..Default::default() };
        let enc = crate::job::make_venc(&s, 64, 48, filmcraft_time::FrameRate::new(30, 1)).expect("the built-in H.264 encoder");
        assert_eq!(enc.encoder_name(), crate::BUILT_IN_ENCODER);
        let lines: Vec<String> = craft_log::testing::captured().into_iter().filter(|l| l.contains("decision: export:")).collect();
        assert!(lines.iter().any(|l| l.contains("H.264 encoded by built-in (software) (no hardware H.264 encoder on this machine)")), "{lines:?}");
    }
}
