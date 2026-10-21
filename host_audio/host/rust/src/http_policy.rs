//! Policy shared by product-owned ASR and minutes HTTP requests. Errors never
//! contain request URLs, response bodies, credentials or meeting content.
use reqwest::{
    blocking::{Client, Response},
    Url,
};
use serde_json::Value;
use std::{fmt, io::Read, time::Duration};

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
pub const MAX_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Configuration,
    Auth,
    RateLimit,
    Http(u16),
    Network,
    Timeout,
    TooLarge,
    Response,
    InvalidResponse,
}

impl Failure {
    pub fn from_reqwest(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::Timeout
        } else if error.is_builder() {
            Self::Configuration
        } else {
            Self::Network
        }
    }
    pub fn from_status(status: u16) -> Self {
        match status {
            401 | 403 => Self::Auth,
            402 | 429 => Self::RateLimit,
            _ => Self::Http(status),
        }
    }
    pub fn retryable(self) -> bool {
        matches!(
            self,
            Self::Network
                | Self::Timeout
                | Self::RateLimit
                | Self::Http(500..=599)
                | Self::Response
        )
    }
}
impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration => f.write_str("configuration invalid"),
            Self::Auth => f.write_str("authentication rejected (HTTP 401/403)"),
            Self::RateLimit => f.write_str("quota or rate limit (HTTP 402/429)"),
            Self::Http(status) => write!(f, "HTTP {status}"),
            Self::Network => f.write_str("network connection failed"),
            Self::Timeout => f.write_str("request timed out"),
            Self::TooLarge => f.write_str("response exceeds size limit"),
            Self::Response => f.write_str("response could not be read completely"),
            Self::InvalidResponse => f.write_str("response is invalid or incomplete"),
        }
    }
}

pub fn endpoint(value: &str) -> Result<Url, Failure> {
    let url = Url::parse(value).map_err(|_| Failure::Configuration)?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(Failure::Configuration);
    }
    Ok(url)
}

pub fn client(timeout: Duration) -> Result<Client, Failure> {
    if timeout.is_zero() || timeout > Duration::from_secs(300) {
        return Err(Failure::Configuration);
    }
    Client::builder()
        .connect_timeout(CONNECT_TIMEOUT.min(timeout))
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(Failure::from_reqwest)
}

pub fn response_json(response: Response, limit: u64) -> Result<Value, Failure> {
    if !response.status().is_success() {
        return Err(Failure::from_status(response.status().as_u16()));
    }
    if limit == 0 || limit > MAX_RESPONSE_BYTES {
        return Err(Failure::Configuration);
    }
    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        return Err(Failure::TooLarge);
    }
    let mut bytes = Vec::new();
    response
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::TimedOut
                || error
                    .get_ref()
                    .and_then(|source| source.downcast_ref::<reqwest::Error>())
                    .is_some_and(reqwest::Error::is_timeout)
            {
                Failure::Timeout
            } else {
                Failure::Response
            }
        })?;
    if bytes.len() as u64 > limit {
        return Err(Failure::TooLarge);
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| Failure::InvalidResponse)?;
    if value.get("error").is_some() {
        return Err(Failure::InvalidResponse);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpListener};

    #[test]
    fn bounded_json_rejects_bad_responses_without_echoing_server_data() {
        for (response, expected) in [
            ("HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\n{\"ok\":1}", Ok(())),
            ("HTTP/1.1 200 OK\r\nContent-Length: 99999999\r\n\r\nsecret", Err(Failure::TooLarge)),
            ("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n12\r\n0123456789secretXYZ\r\n0\r\n\r\n", Err(Failure::TooLarge)),
            ("HTTP/1.1 401 Unauthorized\r\nContent-Length: 99999999\r\n\r\nsecret", Err(Failure::Auth)),
            ("HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\nsecret", Err(Failure::InvalidResponse)),
        ] {
            let listener=TcpListener::bind("127.0.0.1:0").unwrap();
            let address=format!("http://{}", listener.local_addr().unwrap());
            let worker=std::thread::spawn(move || { let (mut stream, _)=listener.accept().unwrap();
                let mut request=[0u8;4096]; let _=stream.read(&mut request); stream.write_all(response.as_bytes()).unwrap(); });
            let response=client(Duration::from_secs(2)).unwrap().get(address).send().unwrap();
            let result=response_json(response,16).map(|_|());
            assert_eq!(result, expected);
            if let Err(error)=result { assert!(!error.to_string().contains("secret")); }
            worker.join().unwrap();
        }
    }
    #[test]
    fn endpoint_and_deadline_configuration_is_explicit() {
        for invalid in [
            "file:///private",
            "https://user:secret@example.org",
            "https://example.org/#secret",
        ] {
            assert!(endpoint(invalid).is_err());
        }
        assert!(endpoint("https://example.org/v1?token=secret").is_ok());
        assert!(client(Duration::ZERO).is_err());
        assert!(client(Duration::from_secs(301)).is_err());
    }
}
