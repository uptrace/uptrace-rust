use core::fmt;
use std::fmt::Display;

use url::Url;

use crate::Error;

#[derive(Default)]
pub struct Dns {
    pub(crate) original: String,
    pub(crate) scheme: String,
    pub(crate) host: String,
    pub(crate) port: Option<u16>,
    pub(crate) project_id: String,
    pub(crate) token: String,
}

impl Dns {
    pub fn otlp_host(&self) -> String {
        if self.host == "uptrace.dev" {
            "otlp.uptrace.dev:4317".into()
        } else {
            match self.port {
                Some(i) => format!("{}:{}", self.host, i),
                None => self.host.clone(),
            }
        }
    }

    pub fn app_addr(&self) -> String {
        if self.host == "uptrace.dev" {
            return "https://app.uptrace.dev".into();
        }

        format!("{}://{}:{}", self.scheme, self.host, 14318)
    }
    pub fn otlp_grpc_addr(&self) -> String {
        if self.host == "uptrace.dev" {
            "https://otlp.uptrace.dev:4317".into()
        } else {
            match self.port {
                Some(port) => format!("{}://{}:{}", self.scheme, self.host, port),
                None => format!("{}://{}", self.scheme, self.host),
            }
        }
    }
}

impl Display for Dns {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.original)
    }
}

impl TryFrom<String> for Dns {
    type Error = Error;
    fn try_from(s: String) -> Result<Dns, Self::Error> {
        if s.is_empty() {
            return Err(Error::EmptyDns);
        }

        let url = Url::parse(&s).map_err(|e| Error::InvalidDns {
            dns: s.clone(),
            reason: e.to_string(),
        })?;
        if url.scheme().is_empty() {
            return Err(Error::InvalidDns {
                dns: s.clone(),
                reason: "schema is not exist".into(),
            });
        }

        let host = if let Some(mut h) = url.host_str() {
            if h == "api.uptrace.dev" {
                h = "uptrace.dev".into();
            }

            h.to_string()
        } else {
            return Err(Error::InvalidDns {
                dns: s.clone(),
                reason: "host is not exist".into(),
            });
        };

        let path = url
            .path_segments()
            .and_then(|x| {
                let path = x.filter(|x| !x.is_empty()).collect::<Vec<&str>>();
                if path.is_empty() {
                    return None;
                } else {
                    Some(path)
                }
            })
            .ok_or_else(|| Error::InvalidDns {
                dns: s.clone(),
                reason: "project id is not exist".into(),
            })?;

        if url.username().is_empty() {
            return Err(Error::InvalidDns {
                dns: s.clone(),
                reason: "token is not exist".into(),
            });
        }

        Ok(Dns {
            original: s,
            scheme: url.scheme().into(),
            host: if host.eq("api.uptrace.dev") {
                "uptrace.dev".into()
            } else {
                host
            },
            port: url.port(),
            token: url.username().into(),
            project_id: path[0].into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::Dns;

    #[test]
    fn valid_dns() {
        let raw = "http://project1_secret_token@localhost:14317/1";
        let dns: Dns = raw.to_string().try_into().unwrap();
        assert_eq!(dns.original, raw.to_string());
        assert_eq!(dns.host, "localhost".to_string());
        assert_eq!(dns.port, Some(14317));
        assert_eq!(dns.scheme, "http".to_string());
        assert_eq!(dns.token, "project1_secret_token".to_string());
        assert_eq!(dns.project_id, "1".to_string());
    }

    #[test]
    fn invalid_dns() {
        let dns = vec![
            "http://project1_secret_token@localhost:14317",
            "http://project1_secret_token@:14317/1",
            "http://localhost:14317/1",
            "project1_secret_token@localhost:14317/1",
        ];
        for i in dns.into_iter() {
            eprintln!("{i}");
            assert!(Dns::try_from(i.to_string()).is_err())
        }
    }

    #[test]
    fn oltp_host() {
        let tables = vec![
            ("https://key@uptrace.dev/1", "otlp.uptrace.dev:4317"),
            ("https://key@api.uptrace.dev/1", "otlp.uptrace.dev:4317"),
            ("https://key@localhost:1234/1", "localhost:1234"),
            (
                "https://AQDan_E_EPe3QAF9fMP0PiVr5UWOu4q5@demo-api.uptrace.dev:4317/1",
                "demo-api.uptrace.dev:4317",
            ),
            ("http://token@localhost:14317/project_id", "localhost:14317"),
            (
                "https://key@uptrace.dev/project_id",
                "otlp.uptrace.dev:4317",
            ),
        ];

        for (i, j) in tables {
            let dns = Dns::try_from(i.to_string()).unwrap();
            assert_eq!(dns.otlp_host(), j);
        }
    }

    #[test]
    fn app_addr() {
        let tables = vec![
            (
                "http://token@localhost:14317/project_id",
                "http://localhost:14318",
            ),
            (
                "https://key@uptrace.dev/project_id",
                "https://app.uptrace.dev",
            ),
        ];

        for (i, j) in tables {
            let dns = Dns::try_from(i.to_string()).unwrap();
            assert_eq!(dns.app_addr(), j);
        }
    }
}
