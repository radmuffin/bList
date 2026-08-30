// src/plus_code.rs
//! Pure-Rust Open Location Code (Plus Code) encoder and decoder.
//! Plus Codes provide location identifiers for any spot on Earth without street addresses.

const CODE_ALPHABET: &[u8; 20] = b"23456789CFGHJMPQRVWX";
const SEPARATOR: char = '+';
const SEPARATOR_POSITION: usize = 8;
const MAX_CODE_LENGTH: usize = 15;

const PAIR_RESOLUTIONS: [f64; 5] = [20.0, 1.0, 0.05, 0.0025, 0.000125];

/// Encode latitude and longitude into a standard 10-character Full Plus Code (e.g. "849VCWC8+R9").
pub fn encode(latitude: f64, longitude: f64, code_length: usize) -> Option<String> {
    if !latitude.is_finite() || !longitude.is_finite() {
        return None;
    }

    let length = code_length.min(10).max(2);
    let length = if length % 2 != 0 { length + 1 } else { length };

    // Normalize latitude to [-90, 90] and longitude to [-180, 180]
    let mut lat = latitude.clamp(-90.0, 90.0);
    let mut lon = longitude;
    while lon < -180.0 {
        lon += 360.0;
    }
    while lon >= 180.0 {
        lon -= 360.0;
    }

    // Latitude 90.0 edge case
    if lat == 90.0 {
        lat = 89.99999999;
    }

    let mut lat_val = lat + 90.0;
    let mut lon_val = lon + 180.0;

    let mut result = String::with_capacity(12);

    let pairs = (length / 2).min(5);
    for i in 0..pairs {
        let res = PAIR_RESOLUTIONS[i];
        let lat_idx = (lat_val / res).floor() as usize;
        let lon_idx = (lon_val / res).floor() as usize;

        let lat_idx = lat_idx.min(19);
        let lon_idx = lon_idx.min(19);

        result.push(CODE_ALPHABET[lat_idx] as char);
        result.push(CODE_ALPHABET[lon_idx] as char);

        lat_val -= (lat_idx as f64) * res;
        lon_val -= (lon_idx as f64) * res;

        if result.len() == SEPARATOR_POSITION {
            result.push(SEPARATOR);
        }
    }

    if result.len() < SEPARATOR_POSITION {
        while result.len() < SEPARATOR_POSITION {
            result.push('0');
        }
        result.push(SEPARATOR);
    } else if !result.contains(SEPARATOR) {
        result.insert(SEPARATOR_POSITION, SEPARATOR);
    }

    Some(result)
}

/// Decode a Full Plus Code into center latitude and longitude coordinates.
pub fn decode(code: &str) -> Option<(f64, f64)> {
    let clean_code = clean_code(code)?;
    if !is_full(&clean_code) {
        return None;
    }

    // Remove separator for processing
    let bare_code: String = clean_code.chars().filter(|&c| c != SEPARATOR && c != '0').collect();

    let mut lat = -90.0;
    let mut lon = -180.0;
    let mut lat_size = 0.0;
    let mut lon_size = 0.0;

    let pairs = (bare_code.len() / 2).min(5);
    for i in 0..pairs {
        let lat_char = bare_code.as_bytes()[i * 2];
        let lon_char = bare_code.as_bytes()[i * 2 + 1];

        let lat_idx = alphabet_index(lat_char)?;
        let lon_idx = alphabet_index(lon_char)?;

        let res = PAIR_RESOLUTIONS[i];
        lat += (lat_idx as f64) * res;
        lon += (lon_idx as f64) * res;
        lat_size = res;
        lon_size = res;
    }

    // Center coordinates in the bounding box
    let center_lat = lat + (lat_size / 2.0);
    let center_lon = lon + (lon_size / 2.0);

    Some((center_lat, center_lon))
}

/// Check if a string is a valid Full Plus Code (e.g. "849VCWC8+R9" or "87G8Q222+22").
pub fn is_full(code: &str) -> bool {
    let code = match clean_code(code) {
        Some(c) => c,
        None => return false,
    };

    if !code.contains(SEPARATOR) {
        return false;
    }
    let sep_idx = match code.find(SEPARATOR) {
        Some(idx) => idx,
        None => return false,
    };

    if sep_idx != SEPARATOR_POSITION {
        return false;
    }

    // First character cannot be padding '0'
    if code.starts_with('0') {
        return false;
    }

    for (i, c) in code.chars().enumerate() {
        if i == SEPARATOR_POSITION {
            if c != SEPARATOR {
                return false;
            }
        } else if c == '0' {
            if i < 2 || i >= SEPARATOR_POSITION {
                return false;
            }
        } else if !CODE_ALPHABET.contains(&(c as u8)) {
            return false;
        }
    }

    true
}

/// Clean input string: uppercase, remove whitespace.
fn clean_code(input: &str) -> Option<String> {
    let trimmed = input.trim().to_uppercase();
    if trimmed.len() < 3 || trimmed.len() > MAX_CODE_LENGTH {
        return None;
    }
    Some(trimmed)
}

fn alphabet_index(b: u8) -> Option<usize> {
    CODE_ALPHABET.iter().position(|&c| c == b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_googleplex() {
        // Googleplex coordinates
        let lat = 37.4220;
        let lon = -122.0841;

        let code = encode(lat, lon, 10).expect("encode plus code");
        assert!(code.starts_with("849V"));
        assert!(code.contains('+'));

        let (dec_lat, dec_lon) = decode(&code).expect("decode plus code");
        assert!((dec_lat - lat).abs() < 0.001);
        assert!((dec_lon - lon).abs() < 0.001);
    }

    #[test]
    fn test_encode_decode_eiffel_tower() {
        // Eiffel Tower: 48.8584, 2.2945 -> 8FW4V8FX+9H or similar
        let lat = 48.8584;
        let lon = 2.2945;

        let code = encode(lat, lon, 10).expect("encode plus code");
        assert!(is_full(&code));

        let (dec_lat, dec_lon) = decode(&code).expect("decode plus code");
        assert!((dec_lat - lat).abs() < 0.001);
        assert!((dec_lon - lon).abs() < 0.001);
    }

    #[test]
    fn test_is_full_validation() {
        assert!(is_full("849VCWC8+R9"));
        assert!(is_full("87G8Q222+22"));
        assert!(is_full("8FW4V8FX+9H"));
        assert!(!is_full("CWC8+R9")); // short code without prefix
        assert!(!is_full("invalid-string"));
        assert!(!is_full("12345678+")); // 1 is not in alphabet
    }
}
