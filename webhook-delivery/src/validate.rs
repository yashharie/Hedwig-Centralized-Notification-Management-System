use url::Url;

/// Why a webhook URL was rejected at the first validation level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlError {
    Malformed,
    UnsupportedScheme,
    MissingHost,
    CredentialsNotAllowed,
}

/// First-level validation: parse the URL and check scheme, host and credentials.
/// This is NOT enough for SSRF protection on its own (see IP checks and connection-time checks).
pub fn parse_webhook_url(input: &str) -> Result<Url, UrlError> {
    let url = Url::parse(input).map_err(|_| UrlError::Malformed)?;

    if url.scheme() != "https" {
        return Err(UrlError::UnsupportedScheme);
    }

    if url.host_str().is_none() {
        return Err(UrlError::MissingHost);
    }

    if !url.username().is_empty() || url.password().is_some() {
        return Err(UrlError::CredentialsNotAllowed);
    }

    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_https_url() {
        let url = parse_webhook_url("https://example.com/hook").unwrap();
        assert_eq!(url.host_str(), Some("example.com"));
    }

    #[test]
    fn rejects_garbage_text() {
        assert_eq!(parse_webhook_url("not a url"), Err(UrlError::Malformed));
    }

    #[test]
    fn rejects_empty_host() {
        assert_eq!(parse_webhook_url("https://"), Err(UrlError::Malformed));
    }

    #[test]
    fn rejects_file_scheme() {
        assert_eq!(
            parse_webhook_url("file:///etc/passwd"),
            Err(UrlError::UnsupportedScheme)
        );
    }

    #[test]
    fn rejects_plain_http() {
        assert_eq!(
            parse_webhook_url("http://example.com/hook"),
            Err(UrlError::UnsupportedScheme)
        );
    }

    #[test]
    fn rejects_credentials_in_url() {
        assert_eq!(
            parse_webhook_url("https://user:secret@example.com/hook"),
            Err(UrlError::CredentialsNotAllowed)
        );
    }
}
