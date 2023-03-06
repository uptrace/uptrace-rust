use core::fmt;
use std::fmt::Display;

use url::Url;

use crate::Error;

#[derive(Default)]
pub struct Dsn {
    pub(crate) original: String,
    pub(crate) scheme: String,
    pub(crate) host: String,
    pub(crate) port: Option<u16>,
    pub(crate) project_id: String,
    pub(crate) token: String,
}

impl Dsn {
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

    #[inline]
    pub(crate) fn is_disabled(&self) -> bool {
        self.project_id == "<project_id>" || self.token == "<token>"
    }
}

impl Display for Dsn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.original)
    }
}

impl TryFrom<String> for Dsn {
    type Error = Error;
    fn try_from(s: String) -> Result<Dsn, Self::Error> {
        if s.is_empty() {
            return Err(Error::EmptyDsn);
        }

        let url = Url::parse(&s).map_err(|e| Error::InvalidDsn {
            dsn: s.clone(),
            reason: e.to_string(),
        })?;
        if url.scheme().is_empty() {
            return Err(Error::InvalidDsn {
                dsn: s,
                reason: "schema is not exist".into(),
            });
        }

        let host = if let Some(mut h) = url.host_str() {
            if h == "api.uptrace.dev" {
                h = "uptrace.dev";
            }

            h.to_string()
        } else {
            return Err(Error::InvalidDsn {
                dsn: s,
                reason: "host is not exist".into(),
            });
        };

        let path = url
            .path_segments()
            .and_then(|x| {
                let path = x.filter(|x| !x.is_empty()).collect::<Vec<&str>>();
                if path.is_empty() {
                    None
                } else {
                    Some(path)
                }
            })
            .ok_or_else(|| Error::InvalidDsn {
                dsn: s.clone(),
                reason: "project id is not exist".into(),
            })?;

        if url.username().is_empty() {
            return Err(Error::InvalidDsn {
                dsn: s.clone(),
                reason: "token is not exist".into(),
            });
        }

        Ok(Dsn {
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

    use super::Dsn;

    #[test]
    fn valid_dsn() {
        let raw = "http://project1_secret_token@localhost:14317/1";
        let dsn: Dsn = raw.to_string().try_into().unwrap();
        assert_eq!(dsn.original, raw.to_string());
        assert_eq!(dsn.host, "localhost".to_string());
        assert_eq!(dsn.port, Some(14317));
        assert_eq!(dsn.scheme, "http".to_string());
        assert_eq!(dsn.token, "project1_secret_token".to_string());
        assert_eq!(dsn.project_id, "1".to_string());
    }

    #[test]
    fn invalid_dsn() {
        let dsn = vec![
            "http://project1_secret_token@localhost:14317",
            "http://project1_secret_token@:14317/1",
            "http://localhost:14317/1",
            "project1_secret_token@localhost:14317/1",
        ];
        for i in dsn.into_iter() {
            eprintln!("{i}");
            assert!(Dsn::try_from(i.to_string()).is_err())
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
            let dsn = Dsn::try_from(i.to_string()).unwrap();
            assert_eq!(dsn.otlp_host(), j);
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
            let dsn = Dsn::try_from(i.to_string()).unwrap();
            assert_eq!(dsn.app_addr(), j);
        }
    }
}
