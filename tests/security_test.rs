// Security tests for validation functions

use brunnylol::validation;

#[tokio::test]
async fn test_validate_url_scheme_blocks_javascript() {
    let result = validation::validate_url_scheme("javascript:alert(1)");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_url_scheme_blocks_data() {
    let result = validation::validate_url_scheme("data:text/html,test");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_url_scheme_blocks_file() {
    let result = validation::validate_url_scheme("file:///etc/passwd");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_url_scheme_allows_http() {
    assert!(validation::validate_url_scheme("http://example.com").is_ok());
}

#[tokio::test]
async fn test_validate_url_scheme_allows_https() {
    assert!(validation::validate_url_scheme("https://example.com").is_ok());
}

#[tokio::test]
async fn test_ssrf_blocks_localhost() {
    let result = validation::validate_url_for_fetch("http://localhost/internal");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_ssrf_blocks_10_network() {
    let result = validation::validate_url_for_fetch("http://10.0.0.1/internal");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_ssrf_blocks_link_local() {
    let result = validation::validate_url_for_fetch("http://169.254.169.254/metadata");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_ssrf_allows_public_domain() {
    assert!(validation::validate_url_for_fetch("https://example.com").is_ok());
}

#[tokio::test]
async fn test_template_blocks_javascript_url() {
    let result = validation::validate_template("javascript:alert({})");
    assert!(result.is_err());
}

// IPv4-mapped IPv6 address bypass prevention tests
#[tokio::test]
async fn test_ssrf_blocks_ipv4_mapped_localhost() {
    // ::ffff:127.0.0.1 is localhost via IPv4-mapped IPv6
    use std::net::IpAddr;
    let ip: IpAddr = "::ffff:127.0.0.1".parse().unwrap();
    let result = validation::validate_resolved_ip(ip);
    assert!(result.is_err(), "Should block IPv4-mapped localhost");
}

#[tokio::test]
async fn test_ssrf_blocks_ipv4_mapped_private_10() {
    // ::ffff:10.0.0.1 is private 10.x network via IPv4-mapped IPv6
    use std::net::IpAddr;
    let ip: IpAddr = "::ffff:10.0.0.1".parse().unwrap();
    let result = validation::validate_resolved_ip(ip);
    assert!(result.is_err(), "Should block IPv4-mapped 10.x.x.x");
}

#[tokio::test]
async fn test_ssrf_blocks_ipv4_mapped_private_192() {
    // ::ffff:192.168.1.1 is private 192.168.x network via IPv4-mapped IPv6
    use std::net::IpAddr;
    let ip: IpAddr = "::ffff:192.168.1.1".parse().unwrap();
    let result = validation::validate_resolved_ip(ip);
    assert!(result.is_err(), "Should block IPv4-mapped 192.168.x.x");
}

#[tokio::test]
async fn test_ssrf_blocks_ipv4_mapped_link_local() {
    // ::ffff:169.254.169.254 is link-local via IPv4-mapped IPv6
    use std::net::IpAddr;
    let ip: IpAddr = "::ffff:169.254.169.254".parse().unwrap();
    let result = validation::validate_resolved_ip(ip);
    assert!(result.is_err(), "Should block IPv4-mapped link-local");
}

#[tokio::test]
async fn test_validate_resolved_ip_allows_public() {
    use std::net::IpAddr;
    let ip: IpAddr = "8.8.8.8".parse().unwrap();
    assert!(validation::validate_resolved_ip(ip).is_ok(), "Should allow public IPv4");

    let ip_v6: IpAddr = "2001:4860:4860::8888".parse().unwrap();
    assert!(validation::validate_resolved_ip(ip_v6).is_ok(), "Should allow public IPv6");
}

#[tokio::test]
async fn test_validate_resolved_ip_blocks_private() {
    use std::net::IpAddr;

    // Test all private ranges
    let ips = vec![
        "127.0.0.1",
        "10.0.0.1",
        "172.16.0.1",
        "192.168.1.1",
        "169.254.169.254",
        "::1",
        "fc00::1",
        "fe80::1",
    ];

    for ip_str in ips {
        let ip: IpAddr = ip_str.parse().unwrap();
        let result = validation::validate_resolved_ip(ip);
        assert!(result.is_err(), "Should block private IP: {}", ip_str);
    }
}

