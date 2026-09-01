// Re-export core security primitives from fly_common
#[allow(unused_imports)]
pub use fly_common::security::{
    build_safe_http_client, is_private_or_restricted_ip, is_restricted_hostname,
    is_restricted_ipv4, is_restricted_ipv6, validate_parsed_url, validate_url_for_ssrf,
};

use reqwest::Url;
use std::net::IpAddr;

/// Validates a URL for SSRF and performs DNS pinning to prevent TOCTOU rebinding attacks.
/// Resolves the hostname, validates all resolved IPs against restricted list,
/// and returns the first valid public IP along with the validated URL.
#[allow(dead_code)] // used in #[cfg(test)] blocks
pub async fn validate_url_with_dns_pin(url_str: &str) -> Result<(Url, IpAddr), String> {
    // First run the basic fly_common checks
    let parsed = validate_url_for_ssrf(url_str)?;

    // Resolve DNS
    let host = parsed.host_str().ok_or("URL must have a host")?;
    let port = parsed.port_or_known_default().unwrap_or(80);
    let lookup = format!("{}:{}", host, port);

    let resolved_ips = tokio::net::lookup_host(lookup)
        .await
        .map_err(|e| format!("DNS resolution failed: {}", e))?;

    let mut first_valid_ip = None;

    for addr in resolved_ips {
        if is_private_or_restricted_ip(addr.ip()) {
            return Err("Restricted IP detected in DNS resolution".to_string());
        }
        if first_valid_ip.is_none() {
            first_valid_ip = Some(addr.ip());
        }
    }

    match first_valid_ip {
        Some(ip) => Ok((parsed, ip)),
        None => Err("No public IP found in DNS resolution".to_string()),
    }
}

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

    #[tokio::test]
    async fn test_validate_url_with_dns_pin_success() {
        let result = validate_url_with_dns_pin("https://example.com").await;
        assert!(result.is_ok());
        let (url, ip) = result.unwrap();
        assert_eq!(url.host_str(), Some("example.com"));
        assert!(!is_private_or_restricted_ip(ip));
    }

    #[tokio::test]
    async fn test_validate_url_with_dns_pin_blocks_localhost() {
        let result = validate_url_with_dns_pin("http://localhost:3000").await;
        assert!(result.is_err());
    }
}
