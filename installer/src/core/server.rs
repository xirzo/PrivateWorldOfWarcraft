use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub const DEFAULT_HOST: &str = "127.0.0.1";
#[allow(dead_code)]
pub const DEFAULT_PORT: u16 = 8085;

/// A game server the client will connect to.
///
/// The default is the local machine (`127.0.0.1`), matching the server-side
/// installer. Playing on a real server requires entering its address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Server {
    pub host: String,
    pub port: Option<u16>,
}

impl Default for Server {
    fn default() -> Self {
        Server {
            host: DEFAULT_HOST.to_string(),
            port: None,
        }
    }
}

impl Server {
    pub fn new(host: &str, port: Option<u16>) -> Self {
        Server {
            host: host.trim().to_string(),
            port,
        }
    }

    /// Parse `host` or `host:port`. IPv6 literals must be bracketed: `[::1]:8085`.
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();
        if input.is_empty() {
            return Err(Error::InvalidServer("empty address".to_string()));
        }

        let (host, port) = if let Some(rest) = input.strip_prefix('[') {
            // Bracket IPv6 literal: "[::1]" or "[::1]:8085"
            let end = rest.find(']').ok_or_else(|| {
                Error::InvalidServer(format!("unterminated IPv6 literal: {input}"))
            })?;
            let host = &rest[..end];
            let after = &rest[end + 1..];
            if after.is_empty() {
                (host.to_string(), None)
            } else if let Some(p) = after.strip_prefix(':') {
                (host.to_string(), Some(parse_port(p, input)?))
            } else {
                return Err(Error::InvalidServer(format!(
                    "invalid IPv6 address: {input}"
                )));
            }
        } else {
            match input.matches(':').count() {
                // Plain hostname / IPv4 address.
                0 => (input.to_string(), None),
                // Exactly one colon: host:port.
                1 => {
                    let (h, p) = input.rsplit_once(':').expect("one colon");
                    (h.to_string(), Some(parse_port(p, input)?))
                }
                // More than one colon: an unbracketed IPv6 literal, no port.
                _ => (input.to_string(), None),
            }
        };

        if host.is_empty() {
            return Err(Error::InvalidServer("host is empty".to_string()));
        }

        Ok(Server::new(&host, port))
    }

    /// Value to write into `set realmlist <value>`.
    pub fn realmlist_value(&self) -> String {
        match self.port {
            Some(p) => format!("{}:{}", self.host, p),
            None => self.host.clone(),
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self.host.as_str(), "127.0.0.1" | "localhost" | "::1")
    }

    #[allow(dead_code)]
    pub fn with_port(self, port: u16) -> Self {
        Server {
            port: Some(port),
            ..self
        }
    }
}

fn parse_port(s: &str, whole: &str) -> Result<u16> {
    let s = s.trim();
    if s.is_empty() {
        return Err(Error::InvalidServer(format!("empty port in: {whole}")));
    }
    let port = s
        .parse::<u16>()
        .map_err(|_| Error::InvalidServer(format!("invalid port `{s}` in: {whole}")))?;
    if port == 0 {
        return Err(Error::InvalidServer(format!(
            "port 0 is not valid: {whole}"
        )));
    }
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_local() {
        let s = Server::default();
        assert_eq!(s.host, "127.0.0.1");
        assert!(s.port.is_none());
        assert!(s.is_local());
        assert_eq!(s.realmlist_value(), "127.0.0.1");
    }

    #[test]
    fn parse_plain_host() {
        let s = Server::parse("play.example.com").unwrap();
        assert_eq!(s.host, "play.example.com");
        assert!(s.port.is_none());
        assert_eq!(s.realmlist_value(), "play.example.com");
    }

    #[test]
    fn parse_host_with_port() {
        let s = Server::parse("192.168.1.50:8085").unwrap();
        assert_eq!(s.host, "192.168.1.50");
        assert_eq!(s.port, Some(8085));
        assert_eq!(s.realmlist_value(), "192.168.1.50:8085");
    }

    #[test]
    fn parse_whitespace_and_default_port() {
        let s = Server::parse("  wow.server.net:3724 ").unwrap();
        assert_eq!(s.host, "wow.server.net");
        assert_eq!(s.port, Some(3724));
    }

    #[test]
    fn parse_ipv6_bracketed() {
        let s = Server::parse("[::1]:8085").unwrap();
        assert_eq!(s.host, "::1");
        assert_eq!(s.port, Some(8085));
        assert!(s.is_local());

        let s = Server::parse("[2001:db8::1]").unwrap();
        assert_eq!(s.host, "2001:db8::1");
        assert!(s.port.is_none());
    }

    #[test]
    fn parse_errors() {
        assert!(Server::parse("").is_err());
        assert!(Server::parse("host:notaport").is_err());
        assert!(Server::parse("host:0").is_err());
        assert!(Server::parse(":8085").is_err());
        assert!(Server::parse("host:99999").is_err());
    }

    #[test]
    fn local_variants() {
        assert!(Server::parse("localhost").unwrap().is_local());
        assert!(Server::parse("::1").unwrap().is_local());
        assert!(!Server::parse("play.server.com").unwrap().is_local());
    }
}
