// Re-export core security primitives from fly_common
#[allow(unused_imports)]
pub use fly_common::security::{
    build_safe_http_client, is_private_or_restricted_ip, is_restricted_hostname,
    is_restricted_ipv4, is_restricted_ipv6, validate_parsed_url, validate_url_for_ssrf,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_restricted_ipv4_addresses() {
        assert!(is_restricted_ipv4(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(is_restricted_ipv4(Ipv4Addr::new(127, 255, 255, 254)));
        assert!(is_restricted_ipv4(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(is_restricted_ipv4(Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_restricted_ipv4(Ipv4Addr::new(192, 168, 1, 1)));
        assert!(is_restricted_ipv4(Ipv4Addr::new(169, 254, 169, 254)));
        assert!(!is_restricted_ipv4(Ipv4Addr::new(8, 8, 8, 8)));
    }

    #[test]
    fn test_restricted_ipv6_addresses() {
        assert!(is_restricted_ipv6(Ipv6Addr::LOCALHOST));
        assert!(is_restricted_ipv6(Ipv6Addr::UNSPECIFIED));
    }

    #[test]
    fn test_restricted_hostnames() {
        assert!(is_restricted_hostname("localhost"));
        assert!(is_restricted_hostname("metadata.google.internal"));
        assert!(!is_restricted_hostname("google.com"));
    }

    #[test]
    fn test_validate_url_ssrf_blocks_private_targets() {
        assert!(validate_url_for_ssrf("http://127.0.0.1").is_err());
        assert!(validate_url_for_ssrf("http://localhost:3000").is_err());
        assert!(validate_url_for_ssrf("http://169.254.169.254/latest/meta-data/").is_err());
        assert!(validate_url_for_ssrf("http://10.0.0.1/").is_err());
        assert!(validate_url_for_ssrf("http://[::1]/").is_err());
        assert!(validate_url_for_ssrf("file:///etc/passwd").is_err());
    }

    #[test]
    fn test_validate_url_allows_valid_public_targets() {
        assert!(validate_url_for_ssrf("https://maps.google.com").is_ok());
        assert!(validate_url_for_ssrf("https://nominatim.openstreetmap.org").is_ok());
    }
}
