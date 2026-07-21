// Ports of termdown's format_seconds / format_seconds_alt / normalize_text.

pub fn format_seconds(seconds: i64, hide_seconds: bool) -> String {
    if seconds <= 60 {
        return seconds.to_string();
    }
    let mut output = String::new();
    let mut seconds = seconds;
    for (period, period_seconds) in [
        ("y", 31557600i64),
        ("d", 86400),
        ("h", 3600),
        ("m", 60),
        ("s", 1),
    ] {
        if seconds >= period_seconds && !(hide_seconds && period == "s") {
            output.push_str(&(seconds / period_seconds).to_string());
            output.push_str(period);
            output.push(' ');
            seconds %= period_seconds;
        }
    }
    output.trim_end().to_string()
}

pub fn format_seconds_alt(seconds: i64, start: i64, hide_seconds: bool) -> String {
    // make sure we always show at least 00:00:00
    let start = start.max(86400);
    let mut output = String::new();
    let total_seconds = seconds;
    let mut seconds = seconds;
    for period_seconds in [31557600i64, 86400, 3600, 60, 1] {
        if hide_seconds && period_seconds == 1 && total_seconds > 60 {
            break;
        }
        let actual_period_value = seconds / period_seconds;
        if actual_period_value > 0 {
            output.push_str(&format!("{:02}:", actual_period_value));
        } else if start > period_seconds || total_seconds > period_seconds {
            output.push_str("00:");
        }
        seconds %= period_seconds;
    }
    output.trim_end_matches(':').to_string()
}

// termdown maps German characters explicitly, then strips combining marks via
// NFD. Full NFD needs Unicode tables; this covers the common precomposed
// Latin letters instead. Unknown non-ASCII characters pass through unchanged.
pub fn normalize_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\u{e4}' => out.push_str("ae"),
            '\u{c4}' => out.push_str("Ae"),
            '\u{f6}' => out.push_str("oe"),
            '\u{d6}' => out.push_str("Oe"),
            '\u{fc}' => out.push_str("ue"),
            '\u{dc}' => out.push_str("Ue"),
            '\u{df}' => out.push_str("ss"),
            '\u{e0}'..='\u{e3}' | '\u{e5}' | '\u{101}' | '\u{103}' | '\u{105}' => out.push('a'),
            '\u{c0}'..='\u{c3}' | '\u{c5}' | '\u{100}' | '\u{102}' | '\u{104}' => out.push('A'),
            '\u{e8}'..='\u{eb}' | '\u{113}' | '\u{115}' | '\u{117}' | '\u{119}' | '\u{11b}' => {
                out.push('e')
            }
            '\u{c8}'..='\u{cb}' | '\u{112}' | '\u{114}' | '\u{116}' | '\u{118}' | '\u{11a}' => {
                out.push('E')
            }
            '\u{ec}'..='\u{ef}' | '\u{129}' | '\u{12b}' | '\u{12d}' | '\u{12f}' => out.push('i'),
            '\u{cc}'..='\u{cf}' | '\u{128}' | '\u{12a}' | '\u{12c}' | '\u{12e}' => out.push('I'),
            '\u{f2}'..='\u{f5}' | '\u{14d}' | '\u{14f}' | '\u{151}' => out.push('o'),
            '\u{d2}'..='\u{d5}' | '\u{14c}' | '\u{14e}' | '\u{150}' => out.push('O'),
            '\u{f9}'..='\u{fb}' | '\u{169}' | '\u{16b}' | '\u{16d}' | '\u{16f}' | '\u{171}' => {
                out.push('u')
            }
            '\u{d9}'..='\u{db}' | '\u{168}' | '\u{16a}' | '\u{16c}' | '\u{16e}' | '\u{170}' => {
                out.push('U')
            }
            '\u{f1}' => out.push('n'),
            '\u{d1}' => out.push('N'),
            '\u{e7}' => out.push('c'),
            '\u{c7}' => out.push('C'),
            '\u{fd}' | '\u{ff}' => out.push('y'),
            '\u{dd}' => out.push('Y'),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_seconds_matches_termdown() {
        assert_eq!(format_seconds(0, false), "0");
        assert_eq!(format_seconds(60, false), "60");
        assert_eq!(format_seconds(61, false), "1m 1s");
        assert_eq!(format_seconds(3600, false), "1h");
        assert_eq!(format_seconds(3661, false), "1h 1m 1s");
        assert_eq!(format_seconds(90061, false), "1d 1h 1m 1s");
        assert_eq!(format_seconds(61, true), "1m");
        assert_eq!(format_seconds(3661, true), "1h 1m");
    }

    #[test]
    fn format_seconds_alt_matches_termdown() {
        assert_eq!(format_seconds_alt(0, 0, false), "00:00:00");
        assert_eq!(format_seconds_alt(59, 100, false), "00:00:59");
        assert_eq!(format_seconds_alt(3661, 3661, false), "01:01:01");
        assert_eq!(format_seconds_alt(90061, 90061, false), "01:01:01:01");
        assert_eq!(format_seconds_alt(61, 0, true), "00:01");
        assert_eq!(format_seconds_alt(59, 0, true), "00:00:59");
    }

    #[test]
    fn normalize_german() {
        assert_eq!(normalize_text("M\u{e4}nner"), "Maenner");
        assert_eq!(normalize_text("Gr\u{f6}\u{df}e"), "Groesse");
        assert_eq!(normalize_text("caf\u{e9}"), "cafe");
        assert_eq!(normalize_text("plain"), "plain");
    }
}
