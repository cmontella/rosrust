use lazy_static::lazy_static;
use std::{collections::HashMap, fmt, sync::RwLock};
use url::{Host, Url};

lazy_static! {
    static ref HOST_ALIASES: RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());
}

/// Error returned when a hostname alias is not a hostname-only value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostAliasError;

impl fmt::Display for HostAliasError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("host aliases must contain a hostname or IP address only")
    }
}

impl std::error::Error for HostAliasError {}

/// Redirects a hostname advertised by a ROS peer to another hostname or IP.
///
/// ROS 1 commonly runs on devices whose advertised hostname is meaningful only
/// on the device itself. Aliases apply process-wide to ROS XML-RPC, TCPROS
/// topic, and TCPROS service connections. Both inputs must be hostnames or IP
/// addresses without a scheme, port, path, user information, or query string.
pub fn set_host_alias(alias: &str, target: &str) -> Result<(), HostAliasError> {
    let alias = parse_host_only(alias)?;
    let target = parse_host_only(target)?;
    HOST_ALIASES
        .write()
        .expect("host alias lock poisoned")
        .insert(alias, target);
    Ok(())
}

pub(crate) fn resolve_host(host: &str) -> String {
    let key = host.to_ascii_lowercase();
    HOST_ALIASES
        .read()
        .expect("host alias lock poisoned")
        .get(&key)
        .cloned()
        .unwrap_or_else(|| host.to_owned())
}

pub(crate) fn rewrite_uri(uri: &str) -> String {
    let had_trailing_slash = uri.ends_with('/');
    let mut parsed = match Url::parse(uri) {
        Ok(parsed) => parsed,
        Err(_) => return uri.to_owned(),
    };
    let target = match parsed.host_str() {
        Some(host) => resolve_host(host),
        None => return uri.to_owned(),
    };
    if parsed.host_str() == Some(target.as_str()) || parsed.set_host(Some(&target)).is_err() {
        return uri.to_owned();
    }
    let mut rewritten = parsed.to_string();
    if !had_trailing_slash && rewritten.ends_with('/') {
        rewritten.pop();
    }
    rewritten
}

fn parse_host_only(value: &str) -> Result<String, HostAliasError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(HostAliasError);
    }
    let candidate = if value.parse::<std::net::Ipv6Addr>().is_ok() {
        format!("[{value}]")
    } else {
        value.to_owned()
    };
    let host = Host::parse(&candidate).map_err(|_| HostAliasError)?;
    if matches!(&host, Host::Domain(domain) if domain.is_empty()) {
        return Err(HostAliasError);
    }
    Ok(host.to_string().to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{rewrite_uri, set_host_alias};

    #[test]
    fn rewrites_http_and_rosrpc_hosts_without_changing_ports() {
        set_host_alias("robot-internal", "10.42.0.1").unwrap();
        assert_eq!(
            rewrite_uri("http://robot-internal:44213/"),
            "http://10.42.0.1:44213/"
        );
        assert_eq!(
            rewrite_uri("rosrpc://robot-internal:55200"),
            "rosrpc://10.42.0.1:55200"
        );
    }

    #[test]
    fn rejects_values_that_are_not_hosts() {
        assert!(set_host_alias("http://robot", "10.42.0.1").is_err());
        assert!(set_host_alias("robot:11311", "10.42.0.1").is_err());
        assert!(set_host_alias("robot", "10.42.0.1/path").is_err());
        assert!(set_host_alias("robot", "user@10.42.0.1").is_err());
    }

    #[test]
    fn accepts_ipv6_targets() {
        set_host_alias("robot-on-v6", "2001:db8::1").unwrap();
        assert_eq!(
            rewrite_uri("http://robot-on-v6:44213/"),
            "http://[2001:db8::1]:44213/"
        );
    }
}
