use std::fmt;
use std::str::FromStr;

use percent_encoding::percent_decode_str;
use url::Url;

use crate::Error;

const CLOUD_HOST: &str = "uptrace.dev";

/// Uptrace data source name, for example, `https://<token>@api.uptrace.dev?grpc=4317`.
///
/// You can find your project DSN in the project settings.
#[derive(Clone, PartialEq, Eq)]
pub struct Dsn {
    original: String,
    scheme: String,
    host: String,
    http_port: Option<u16>,
    grpc_port: Option<u16>,
    token: String,
}

impl Dsn {
    /// Parses a DSN string.
    pub fn parse(s: &str) -> Result<Self, Error> {
        s.parse()
    }

    /// URL scheme, `http` or `https`.
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    /// Normalized host name; `api.uptrace.dev` is reported as `uptrace.dev`.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Project token used to authenticate requests.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Port of the OTLP/HTTP endpoint and the Uptrace UI.
    pub fn http_port(&self) -> Option<u16> {
        self.http_port
    }

    /// Port of the OTLP/gRPC endpoint.
    pub fn grpc_port(&self) -> Option<u16> {
        self.grpc_port
    }

    /// Whether the DSN is a placeholder copied from the docs, e.g. `https://<token>@uptrace.dev`.
    pub fn is_dummy(&self) -> bool {
        self.token == "<token>"
    }

    fn is_cloud(&self) -> bool {
        self.host == CLOUD_HOST
    }

    /// Base URL of the Uptrace UI, e.g. `https://app.uptrace.dev`.
    pub fn site_url(&self) -> String {
        if self.is_cloud() {
            return "https://app.uptrace.dev".into();
        }
        self.url_with_port(self.http_port)
    }

    /// OTLP/HTTP endpoint without the signal path, e.g. `https://api.uptrace.dev:443`.
    pub fn otlp_http_endpoint(&self) -> String {
        if self.is_cloud() {
            return "https://api.uptrace.dev:443".into();
        }
        self.url_with_port(self.http_port)
    }

    /// OTLP/gRPC endpoint, e.g. `https://api.uptrace.dev:4317`.
    pub fn otlp_grpc_endpoint(&self) -> String {
        if self.is_cloud() {
            return "https://api.uptrace.dev:4317".into();
        }
        self.url_with_port(self.grpc_port)
    }

    fn url_with_port(&self, port: Option<u16>) -> String {
        match port {
            Some(port) => format!("{}://{}:{}", self.scheme, self.host, port),
            None => format!("{}://{}", self.scheme, self.host),
        }
    }
}

impl FromStr for Dsn {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(Error::EmptyDsn);
        }

        let invalid = |reason: &str| Error::InvalidDsn {
            dsn: s.to_string(),
            reason: reason.to_string(),
        };

        let url = Url::parse(s).map_err(|err| invalid(&err.to_string()))?;

        let host = match url.host_str() {
            Some("") | None => return Err(invalid("host is missing")),
            Some("api.uptrace.dev") => CLOUD_HOST.to_string(),
            Some(host) => host.to_string(),
        };
        // The url crate percent-encodes the username, e.g. `<token>` placeholders.
        let token = percent_decode_str(url.username())
            .decode_utf8()
            .map_err(|_| invalid("token is not valid UTF-8"))?
            .into_owned();
        if token.is_empty() {
            return Err(invalid("token is missing"));
        }

        let mut http_port = url.port_or_known_default();
        let grpc_port = match url.query_pairs().find(|(k, _)| k == "grpc") {
            Some((_, port)) => Some(
                port.parse::<u16>()
                    .map_err(|_| invalid("grpc port is not a number"))?,
            ),
            // Without an explicit gRPC port, the DSN port is the gRPC port,
            // and the self-hosted default 14317 pairs with HTTP port 14318.
            None => {
                let grpc = http_port.or(Some(4317));
                if http_port == Some(14317) {
                    http_port = Some(14318);
                }
                grpc
            }
        };

        Ok(Dsn {
            original: s.to_string(),
            scheme: url.scheme().to_string(),
            host,
            http_port,
            grpc_port,
            token,
        })
    }
}

impl TryFrom<&str> for Dsn {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl TryFrom<String> for Dsn {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl fmt::Display for Dsn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.original)
    }
}

// The DSN contains a secret token, so don't leak it via `{:?}`.
impl fmt::Debug for Dsn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dsn")
            .field("scheme", &self.scheme)
            .field("host", &self.host)
            .field("http_port", &self.http_port)
            .field("grpc_port", &self.grpc_port)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        // dsn, grpc endpoint, http endpoint, site url
        let tests = [
            (
                "https://token@uptrace.dev/1",
                "https://api.uptrace.dev:4317",
                "https://api.uptrace.dev:443",
                "https://app.uptrace.dev",
            ),
            (
                "https://token@api.uptrace.dev/1",
                "https://api.uptrace.dev:4317",
                "https://api.uptrace.dev:443",
                "https://app.uptrace.dev",
            ),
            (
                "https://token@demo.uptrace.dev/1?grpc=4317",
                "https://demo.uptrace.dev:4317",
                "https://demo.uptrace.dev:443",
                "https://demo.uptrace.dev:443",
            ),
            (
                "https://token@localhost:1234/1",
                "https://localhost:1234",
                "https://localhost:1234",
                "https://localhost:1234",
            ),
            (
                "http://token@localhost:14317/project_id",
                "http://localhost:14317",
                "http://localhost:14318",
                "http://localhost:14318",
            ),
            (
                "https://AQDan_E_EPe3QAF9fMP0PiVr5UWOu4q5@demo-api.uptrace.dev:4317/1",
                "https://demo-api.uptrace.dev:4317",
                "https://demo-api.uptrace.dev:4317",
                "https://demo-api.uptrace.dev:4317",
            ),
            (
                "http://Qcn7rcwWO_w0ePo7WmeUtw@localhost:14318?grpc=14317",
                "http://localhost:14317",
                "http://localhost:14318",
                "http://localhost:14318",
            ),
        ];

        for (raw, grpc, http, site_url) in tests {
            let dsn = Dsn::parse(raw).unwrap();
            assert_eq!(dsn.to_string(), raw);
            assert_eq!(dsn.otlp_grpc_endpoint(), grpc, "{raw}");
            assert_eq!(dsn.otlp_http_endpoint(), http, "{raw}");
            assert_eq!(dsn.site_url(), site_url, "{raw}");
        }
    }

    #[test]
    fn fields() {
        let dsn = Dsn::parse("http://project1_secret@localhost:14317/1").unwrap();
        assert_eq!(dsn.scheme(), "http");
        assert_eq!(dsn.host(), "localhost");
        assert_eq!(dsn.token(), "project1_secret");
        assert_eq!(dsn.http_port(), Some(14318));
        assert_eq!(dsn.grpc_port(), Some(14317));
        assert!(!dsn.is_dummy());
        assert!(Dsn::parse("https://<token>@uptrace.dev")
            .unwrap()
            .is_dummy());
    }

    #[test]
    fn invalid() {
        assert!(matches!(Dsn::parse(""), Err(Error::EmptyDsn)));

        for raw in [
            "http://localhost:14317/1",
            "project1_secret@localhost:14317/1",
            "http://token@localhost?grpc=abc",
        ] {
            assert!(
                matches!(Dsn::parse(raw), Err(Error::InvalidDsn { .. })),
                "{raw}"
            );
        }
    }

    #[test]
    fn debug_hides_token() {
        let dsn = Dsn::parse("https://secret@uptrace.dev/1").unwrap();
        assert!(!format!("{dsn:?}").contains("secret"));
    }
}
