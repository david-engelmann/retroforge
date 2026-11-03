//! Shared constructions: argument and path text in
//! CPython's filesystem decoding, the messages the
//! record's and the wall's phrases are built from,
//! the strerror table the OS error phrases read,
//! and the Unicode tables the argument matcher and
//! the sentence rule read.

use flighttape::value::Text;

/// Decode OS bytes the way CPython decodes file system
/// and argument strings: valid UTF-8 decodes normally,
/// and each byte that cannot decode becomes the
/// surrogateescape unit 0xDC00 plus the byte.
pub fn fsdecode_units(bytes: &[u8]) -> Vec<u16> {
    let mut units = Vec::new();
    let mut rest = bytes;
    loop {
        match std::str::from_utf8(rest) {
            Ok(text) => {
                units.extend(text.encode_utf16());
                return units;
            }
            Err(error) => {
                let valid = error.valid_up_to();
                let head = std::str::from_utf8(&rest[..valid]).expect("the valid prefix");
                units.extend(head.encode_utf16());
                match error.error_len() {
                    Some(len) => {
                        for byte in &rest[valid..valid + len] {
                            units.push(0xDC00 + *byte as u16);
                        }
                        rest = &rest[valid + len..];
                    }
                    None => {
                        for byte in &rest[valid..] {
                            units.push(0xDC00 + *byte as u16);
                        }
                        return units;
                    }
                }
            }
        }
    }
}

/// Whether a standard stream was closed at launch.
/// The Rust runtime repairs a missing standard
/// descriptor before main runs by opening /dev/null
/// over it read-write; that substitution is the
/// only ordinary source of a read-write /dev/null
/// on a standard descriptor, and /proc carries both
/// the target and the access mode. A stream the
/// caller aimed at /dev/null itself is opened
/// read-only or write-only and reads as present, as
/// does any file, pipe, or socket.
pub fn closed_at_launch(fd: u32) -> bool {
    let target = match std::fs::read_link(format!("/proc/self/fd/{fd}")) {
        Ok(target) => target,
        Err(_) => return false,
    };
    if target != std::path::Path::new("/dev/null") {
        return false;
    }
    let info = match std::fs::read_to_string(format!("/proc/self/fdinfo/{fd}")) {
        Ok(info) => info,
        Err(_) => return false,
    };
    for line in info.lines() {
        if let Some(flags) = line.strip_prefix("flags:") {
            if let Ok(value) = i64::from_str_radix(flags.trim(), 8) {
                return value & 0o3 == 2;
            }
        }
    }
    false
}

/// The scalar a surrogate pair encodes.
pub fn decode_pair(high: u16, low: u16) -> u32 {
    0x10000 + (((high as u32) - 0xD800) << 10) + ((low as u32) - 0xDC00)
}

/// The decoded scalars of a unit string: surrogate
/// pairs as their astral scalar, lone units at face
/// value. The one decode walk every module shares.
pub fn scalars_of(units: &[u16]) -> Vec<u32> {
    let mut out = Vec::with_capacity(units.len());
    let mut index = 0;
    while index < units.len() {
        let span = scalar_span(units, index);
        if span == 2 {
            out.push(decode_pair(units[index], units[index + 1]));
        } else {
            out.push(units[index] as u32);
        }
        index += span;
    }
    out
}

/// How many units the scalar starting at `index`
/// spans: two for a surrogate pair, one otherwise.
pub fn scalar_span(units: &[u16], index: usize) -> usize {
    let unit = units[index];
    if (0xD800..=0xDBFF).contains(&unit)
        && index + 1 < units.len()
        && (0xDC00..=0xDFFF).contains(&units[index + 1])
    {
        2
    } else {
        1
    }
}

/// The unit index at which scalar number
/// `scalar_index` begins (the unit length when the
/// index runs past the last scalar).
pub fn unit_offset(units: &[u16], scalar_index: usize) -> usize {
    let mut unit_at = 0;
    let mut seen = 0;
    while seen < scalar_index && unit_at < units.len() {
        unit_at += scalar_span(units, unit_at);
        seen += 1;
    }
    unit_at
}

/// The inverse: encode text units back to OS bytes,
/// mapping surrogateescape units back to their bytes.
/// A lone surrogate outside the escape range cannot
/// be encoded; it renders as the UTF-8 replacement,
/// a case argument strings from the OS never reach
/// (their surrogates come only from fsdecode).
#[cfg(unix)]
pub fn fsencode_bytes(units: &[u16]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < units.len() {
        let unit = units[index];
        if (0xDC80..=0xDCFF).contains(&unit) {
            out.push((unit - 0xDC00) as u8);
            index += 1;
        } else if (0xD800..=0xDBFF).contains(&unit)
            && index + 1 < units.len()
            && (0xDC00..=0xDFFF).contains(&units[index + 1])
        {
            let scalar = decode_pair(unit, units[index + 1]);
            let mut buf = [0u8; 4];
            out.extend_from_slice(
                char::from_u32(scalar)
                    .expect("a valid pair")
                    .encode_utf8(&mut buf)
                    .as_bytes(),
            );
            index += 2;
        } else if (0xD800..=0xDFFF).contains(&unit) {
            out.extend_from_slice("\u{FFFD}".as_bytes());
            index += 1;
        } else {
            let mut buf = [0u8; 4];
            out.extend_from_slice(
                char::from_u32(unit as u32)
                    .expect("a BMP scalar")
                    .encode_utf8(&mut buf)
                    .as_bytes(),
            );
            index += 1;
        }
    }
    out
}

/// Render text for a stderr message the way CPython's
/// stderr does (its error handler is backslashreplace):
/// valid text passes through, and every lone surrogate
/// becomes its \uXXXX escape.
pub fn stderr_render(units: &[u16]) -> String {
    let mut out = String::new();
    let mut index = 0;
    while index < units.len() {
        let unit = units[index];
        if (0xD800..=0xDBFF).contains(&unit)
            && index + 1 < units.len()
            && (0xDC00..=0xDFFF).contains(&units[index + 1])
        {
            let scalar = decode_pair(unit, units[index + 1]);
            out.push(char::from_u32(scalar).expect("a valid pair"));
            index += 2;
        } else if (0xD800..=0xDFFF).contains(&unit) {
            out.push_str(&format!("\\u{unit:04x}"));
            index += 1;
        } else {
            out.push(char::from_u32(unit as u32).expect("a BMP scalar"));
            index += 1;
        }
    }
    out
}

/// A message under construction, held as text units
/// so a name the Python inserts raw (a claim id, a
/// path) keeps its exact units until the message is
/// rendered for its destination: stderr renders
/// backslashreplace, stdout surrogateescape.
#[derive(Clone, Debug, PartialEq)]
pub struct Msg(pub Vec<u16>);

// new is the constructor every caller uses; a
// Default implementation stood here unused and
// went in the Round 3 prune.
#[allow(clippy::new_without_default)]
impl Msg {
    pub fn new() -> Msg {
        Msg(Vec::new())
    }

    pub fn str(text: &str) -> Msg {
        let mut msg = Msg::new();
        msg.push_str(text);
        msg
    }

    pub fn push_str(&mut self, text: &str) {
        self.0.extend(text.encode_utf16());
    }

    pub fn push_text(&mut self, text: &Text) {
        self.0.extend_from_slice(text.units());
    }

    pub fn push_units(&mut self, units: &[u16]) {
        self.0.extend_from_slice(units);
    }

    pub fn push_msg(&mut self, other: &Msg) {
        self.0.extend_from_slice(&other.0);
    }

    pub fn render_stderr(&self) -> String {
        stderr_render(&self.0)
    }
}

/// Join messages with a separator, the way the
/// Python joins defect strings.
pub fn join_msgs(items: &[Msg], separator: &str) -> Msg {
    let mut out = Msg::new();
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push_str(separator);
        }
        out.push_msg(item);
    }
    out
}

/// The bare strerror text for an OS error, as the
/// Python's OSError.strerror reads. One table serves
/// the store's phrases and the CLI's "[Errno N]"
/// renderings. Errnos outside the table fall back
/// to the standard library's rendering, a named
/// deviation.
pub fn strerror_text(err: &std::io::Error) -> String {
    match err.raw_os_error() {
        Some(errno) => {
            let text = match errno {
                1 => "Operation not permitted",
                2 => "No such file or directory",
                13 => "Permission denied",
                20 => "Not a directory",
                21 => "Is a directory",
                24 => "Too many open files",
                28 => "No space left on device",
                30 => "Read-only file system",
                32 => "Broken pipe",
                36 => "File name too long",
                40 => "Too many levels of symbolic links",
                _ => "",
            };
            if text.is_empty() {
                err.to_string()
            } else {
                text.to_string()
            }
        }
        None => err.to_string(),
    }
}

/// The OS path for a normalized display path: the
/// fsencoding of its units.
pub fn os_path_from_units(units: &[u16]) -> std::path::PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        std::path::PathBuf::from(std::ffi::OsString::from_vec(fsencode_bytes(units)))
    }
    #[cfg(not(unix))]
    {
        std::path::PathBuf::from(String::from_utf16_lossy(units))
    }
}

/// The first code point of each block of ten
/// Unicode decimal digits (the Nd category): every
/// block runs 0 to 9 from its start.
const DECIMAL_DIGIT_STARTS: [u32; 68] = [
    0x30, 0x660, 0x6F0, 0x7C0, 0x966, 0x9E6, 0xA66, 0xAE6, 0xB66, 0xBE6, 0xC66, 0xCE6, 0xD66,
    0xDE6, 0xE50, 0xED0, 0xF20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80, 0x1A90,
    0x1B50, 0x1BB0, 0x1C40, 0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0, 0xFF10,
    0x104A0, 0x10D30, 0x11066, 0x110F0, 0x11136, 0x111D0, 0x112F0, 0x11450, 0x114D0, 0x11650,
    0x116C0, 0x11730, 0x118E0, 0x11950, 0x11C50, 0x11D50, 0x11DA0, 0x11F50, 0x16A60, 0x16AC0,
    0x16B50, 0x1D7CE, 0x1D7D8, 0x1D7E2, 0x1D7EC, 0x1D7F6, 0x1E140, 0x1E2F0, 0x1E4F0, 0x1E950,
    0x1FBF0,
];

/// A scalar's Unicode decimal digit value, where it
/// has one: the digit set CPython's float() and
/// argparse's negative number matcher read.
pub fn decimal_digit(scalar: u32) -> Option<u32> {
    let at = DECIMAL_DIGIT_STARTS.partition_point(|start| *start <= scalar);
    if at == 0 {
        return None;
    }
    let start = DECIMAL_DIGIT_STARTS[at - 1];
    let offset = scalar - start;
    if offset < 10 {
        Some(offset)
    } else {
        None
    }
}

/// The first index of a unit subslice, for the
/// defect tail the add flow raises.
pub fn find_subslice(haystack: &[u16], needle: &[u16]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// CPython's strip set: the characters str.strip()
/// removes, the ASCII whitespace and file separators
/// plus the Unicode space characters.
fn py_is_space(scalar: u32) -> bool {
    matches!(scalar,
        0x09..=0x0D
        | 0x1C..=0x1F
        | 0x20
        | 0x85
        | 0xA0
        | 0x1680
        | 0x2000..=0x200A
        | 0x2028
        | 0x2029
        | 0x202F
        | 0x205F
        | 0x3000)
}

/// Text units with the strip set removed from both
/// ends, the way the Python's str.strip() reads.
pub fn py_strip_units(units: &[u16]) -> &[u16] {
    let scalars = scalars_of(units);
    let mut points: Vec<(usize, u32)> = Vec::with_capacity(scalars.len());
    let mut at = 0;
    for scalar in &scalars {
        points.push((at, *scalar));
        at += scalar_span(units, at);
    }
    let mut start = 0;
    while start < points.len() && py_is_space(points[start].1) {
        start += 1;
    }
    let mut tail = points.len();
    while tail > start && py_is_space(points[tail - 1].1) {
        tail -= 1;
    }
    if start >= tail {
        return &units[units.len()..units.len()];
    }
    let from = points[start].0;
    let to = if tail < points.len() {
        points[tail].0
    } else {
        units.len()
    };
    &units[from..to]
}

/// CPython's str.isalnum() for one code point, the
/// sentence rule's content test. A full-codepoint
/// sweep of CPython 3.12's isalnum against this
/// toolchain's char::is_alphanumeric (Round 3)
/// found the Python set a strict subset of the
/// Rust one, with 6,499 codepoints in 275 ranges
/// where Rust reads a mark or an enclosed letter
/// as alphanumeric and CPython does not. The
/// semantics here are Rust's test minus the swept
/// ranges below.
pub fn py_is_alnum(scalar: u32) -> bool {
    if scalar < 128 {
        return (scalar as u8).is_ascii_alphanumeric();
    }
    match char::from_u32(scalar) {
        Some(ch) => ch.is_alphanumeric() && !in_not_alnum_ranges(scalar),
        None => false,
    }
}

fn in_not_alnum_ranges(scalar: u32) -> bool {
    NOT_ALNUM_RANGES
        .binary_search_by(|&(start, end)| {
            if scalar < start {
                std::cmp::Ordering::Greater
            } else if scalar > end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// The swept ranges where char::is_alphanumeric
/// holds and CPython 3.12's str.isalnum does not.
const NOT_ALNUM_RANGES: &[(u32, u32)] = &[
    (0x345, 0x345),
    (0x363, 0x36F),
    (0x5B0, 0x5BD),
    (0x5BF, 0x5BF),
    (0x5C1, 0x5C2),
    (0x5C4, 0x5C5),
    (0x5C7, 0x5C7),
    (0x610, 0x61A),
    (0x64B, 0x657),
    (0x659, 0x65F),
    (0x670, 0x670),
    (0x6D6, 0x6DC),
    (0x6E1, 0x6E4),
    (0x6E7, 0x6E8),
    (0x6ED, 0x6ED),
    (0x711, 0x711),
    (0x730, 0x73F),
    (0x7A6, 0x7B0),
    (0x816, 0x817),
    (0x81B, 0x823),
    (0x825, 0x827),
    (0x829, 0x82C),
    (0x897, 0x897),
    (0x8D4, 0x8DF),
    (0x8E3, 0x8E9),
    (0x8F0, 0x903),
    (0x93A, 0x93B),
    (0x93E, 0x94C),
    (0x94E, 0x94F),
    (0x955, 0x957),
    (0x962, 0x963),
    (0x981, 0x983),
    (0x9BE, 0x9C4),
    (0x9C7, 0x9C8),
    (0x9CB, 0x9CC),
    (0x9D7, 0x9D7),
    (0x9E2, 0x9E3),
    (0xA01, 0xA03),
    (0xA3E, 0xA42),
    (0xA47, 0xA48),
    (0xA4B, 0xA4C),
    (0xA51, 0xA51),
    (0xA70, 0xA71),
    (0xA75, 0xA75),
    (0xA81, 0xA83),
    (0xABE, 0xAC5),
    (0xAC7, 0xAC9),
    (0xACB, 0xACC),
    (0xAE2, 0xAE3),
    (0xAFA, 0xAFC),
    (0xB01, 0xB03),
    (0xB3E, 0xB44),
    (0xB47, 0xB48),
    (0xB4B, 0xB4C),
    (0xB56, 0xB57),
    (0xB62, 0xB63),
    (0xB82, 0xB82),
    (0xBBE, 0xBC2),
    (0xBC6, 0xBC8),
    (0xBCA, 0xBCC),
    (0xBD7, 0xBD7),
    (0xC00, 0xC04),
    (0xC3E, 0xC44),
    (0xC46, 0xC48),
    (0xC4A, 0xC4C),
    (0xC55, 0xC56),
    (0xC62, 0xC63),
    (0xC81, 0xC83),
    (0xCBE, 0xCC4),
    (0xCC6, 0xCC8),
    (0xCCA, 0xCCC),
    (0xCD5, 0xCD6),
    (0xCE2, 0xCE3),
    (0xCF3, 0xCF3),
    (0xD00, 0xD03),
    (0xD3E, 0xD44),
    (0xD46, 0xD48),
    (0xD4A, 0xD4C),
    (0xD57, 0xD57),
    (0xD62, 0xD63),
    (0xD81, 0xD83),
    (0xDCF, 0xDD4),
    (0xDD6, 0xDD6),
    (0xDD8, 0xDDF),
    (0xDF2, 0xDF3),
    (0xE31, 0xE31),
    (0xE34, 0xE3A),
    (0xE4D, 0xE4D),
    (0xEB1, 0xEB1),
    (0xEB4, 0xEB9),
    (0xEBB, 0xEBC),
    (0xECD, 0xECD),
    (0xF71, 0xF83),
    (0xF8D, 0xF97),
    (0xF99, 0xFBC),
    (0x102B, 0x1036),
    (0x1038, 0x1038),
    (0x103B, 0x103E),
    (0x1056, 0x1059),
    (0x105E, 0x1060),
    (0x1062, 0x1064),
    (0x1067, 0x106D),
    (0x1071, 0x1074),
    (0x1082, 0x108D),
    (0x108F, 0x108F),
    (0x109A, 0x109D),
    (0x1712, 0x1713),
    (0x1732, 0x1733),
    (0x1752, 0x1753),
    (0x1772, 0x1773),
    (0x17B6, 0x17C8),
    (0x1885, 0x1886),
    (0x18A9, 0x18A9),
    (0x1920, 0x192B),
    (0x1930, 0x1938),
    (0x1A17, 0x1A1B),
    (0x1A55, 0x1A5E),
    (0x1A61, 0x1A74),
    (0x1ABF, 0x1AC0),
    (0x1ACC, 0x1ACE),
    (0x1B00, 0x1B04),
    (0x1B35, 0x1B43),
    (0x1B80, 0x1B82),
    (0x1BA1, 0x1BA9),
    (0x1BAC, 0x1BAD),
    (0x1BE7, 0x1BF1),
    (0x1C24, 0x1C36),
    (0x1C89, 0x1C8A),
    (0x1DD3, 0x1DF4),
    (0x24B6, 0x24E9),
    (0x2DE0, 0x2DFF),
    (0xA674, 0xA67B),
    (0xA69E, 0xA69F),
    (0xA7CB, 0xA7CD),
    (0xA7DA, 0xA7DC),
    (0xA802, 0xA802),
    (0xA80B, 0xA80B),
    (0xA823, 0xA827),
    (0xA880, 0xA881),
    (0xA8B4, 0xA8C3),
    (0xA8C5, 0xA8C5),
    (0xA8FF, 0xA8FF),
    (0xA926, 0xA92A),
    (0xA947, 0xA952),
    (0xA980, 0xA983),
    (0xA9B4, 0xA9BF),
    (0xA9E5, 0xA9E5),
    (0xAA29, 0xAA36),
    (0xAA43, 0xAA43),
    (0xAA4C, 0xAA4D),
    (0xAA7B, 0xAA7D),
    (0xAAB0, 0xAAB0),
    (0xAAB2, 0xAAB4),
    (0xAAB7, 0xAAB8),
    (0xAABE, 0xAABE),
    (0xAAEB, 0xAAEF),
    (0xAAF5, 0xAAF5),
    (0xABE3, 0xABEA),
    (0xFB1E, 0xFB1E),
    (0x10376, 0x1037A),
    (0x105C0, 0x105F3),
    (0x10A01, 0x10A03),
    (0x10A05, 0x10A06),
    (0x10A0C, 0x10A0F),
    (0x10D24, 0x10D27),
    (0x10D40, 0x10D65),
    (0x10D69, 0x10D69),
    (0x10D6F, 0x10D85),
    (0x10EAB, 0x10EAC),
    (0x10EC2, 0x10EC4),
    (0x10EFC, 0x10EFC),
    (0x11000, 0x11002),
    (0x11038, 0x11045),
    (0x11073, 0x11074),
    (0x11080, 0x11082),
    (0x110B0, 0x110B8),
    (0x110C2, 0x110C2),
    (0x11100, 0x11102),
    (0x11127, 0x11132),
    (0x11145, 0x11146),
    (0x11180, 0x11182),
    (0x111B3, 0x111BF),
    (0x111CE, 0x111CF),
    (0x1122C, 0x11234),
    (0x11237, 0x11237),
    (0x1123E, 0x1123E),
    (0x11241, 0x11241),
    (0x112DF, 0x112E8),
    (0x11300, 0x11303),
    (0x1133E, 0x11344),
    (0x11347, 0x11348),
    (0x1134B, 0x1134C),
    (0x11357, 0x11357),
    (0x11362, 0x11363),
    (0x11380, 0x11389),
    (0x1138B, 0x1138B),
    (0x1138E, 0x1138E),
    (0x11390, 0x113B5),
    (0x113B7, 0x113C0),
    (0x113C2, 0x113C2),
    (0x113C5, 0x113C5),
    (0x113C7, 0x113CA),
    (0x113CC, 0x113CD),
    (0x113D1, 0x113D1),
    (0x113D3, 0x113D3),
    (0x11435, 0x11441),
    (0x11443, 0x11445),
    (0x114B0, 0x114C1),
    (0x115AF, 0x115B5),
    (0x115B8, 0x115BE),
    (0x115DC, 0x115DD),
    (0x11630, 0x1163E),
    (0x11640, 0x11640),
    (0x116AB, 0x116B5),
    (0x116D0, 0x116E3),
    (0x1171D, 0x1172A),
    (0x1182C, 0x11838),
    (0x11930, 0x11935),
    (0x11937, 0x11938),
    (0x1193B, 0x1193C),
    (0x11940, 0x11940),
    (0x11942, 0x11942),
    (0x119D1, 0x119D7),
    (0x119DA, 0x119DF),
    (0x119E4, 0x119E4),
    (0x11A01, 0x11A0A),
    (0x11A35, 0x11A39),
    (0x11A3B, 0x11A3E),
    (0x11A51, 0x11A5B),
    (0x11A8A, 0x11A97),
    (0x11BC0, 0x11BE0),
    (0x11BF0, 0x11BF9),
    (0x11C2F, 0x11C36),
    (0x11C38, 0x11C3E),
    (0x11C92, 0x11CA7),
    (0x11CA9, 0x11CB6),
    (0x11D31, 0x11D36),
    (0x11D3A, 0x11D3A),
    (0x11D3C, 0x11D3D),
    (0x11D3F, 0x11D41),
    (0x11D43, 0x11D43),
    (0x11D47, 0x11D47),
    (0x11D8A, 0x11D8E),
    (0x11D90, 0x11D91),
    (0x11D93, 0x11D96),
    (0x11EF3, 0x11EF6),
    (0x11F00, 0x11F01),
    (0x11F03, 0x11F03),
    (0x11F34, 0x11F3A),
    (0x11F3E, 0x11F40),
    (0x13460, 0x143FA),
    (0x16100, 0x1612E),
    (0x16130, 0x16139),
    (0x16D40, 0x16D6C),
    (0x16D70, 0x16D79),
    (0x16F4F, 0x16F4F),
    (0x16F51, 0x16F87),
    (0x16F8F, 0x16F92),
    (0x16FF0, 0x16FF1),
    (0x18CFF, 0x18CFF),
    (0x1BC9E, 0x1BC9E),
    (0x1CCF0, 0x1CCF9),
    (0x1E000, 0x1E006),
    (0x1E008, 0x1E018),
    (0x1E01B, 0x1E021),
    (0x1E023, 0x1E024),
    (0x1E026, 0x1E02A),
    (0x1E08F, 0x1E08F),
    (0x1E5D0, 0x1E5ED),
    (0x1E5F0, 0x1E5FA),
    (0x1E947, 0x1E947),
    (0x1F130, 0x1F149),
    (0x1F150, 0x1F169),
    (0x1F170, 0x1F189),
    (0x2EBF0, 0x2EE5D),
];

/// The "[Errno N] strerror: 'path'" rendering for an
/// OS error raised against a path, pathlib's form.
pub fn os_error_with_path(error: &std::io::Error, display: &Msg) -> Msg {
    let mut message = Msg::new();
    let errno = error.raw_os_error().unwrap_or(0);
    message.push_str(&format!("[Errno {}] ", errno));
    message.push_str(&strerror_text(error));
    message.push_str(": ");
    let text = Text::from_units(display.0.clone());
    message.push_str(&crate::pyrepr::repr_text(&text));
    message
}
