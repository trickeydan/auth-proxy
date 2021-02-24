use hyper::header::{HeaderMap, HeaderName, HeaderValue, HOST, VIA};
use lazy_static::lazy_static;

use hyper::{Request, Response, Uri};
use std::net::IpAddr;

use super::config;
use super::SERVER_VIA;

fn is_hop_header(name: &str) -> bool {
    use unicase::Ascii;

    // A list of the headers, using `unicase` to help us compare without
    // worrying about the case, and `lazy_static!` to prevent reallocation
    // of the vector.
    lazy_static! {
        static ref HOP_HEADERS: Vec<Ascii<&'static str>> = vec![
            Ascii::new("Connection"),
            Ascii::new("Keep-Alive"),
            Ascii::new("Proxy-Authenticate"),
            Ascii::new("Proxy-Authorization"),
            Ascii::new("Te"),
            Ascii::new("Trailers"),
            Ascii::new("Transfer-Encoding"),
            Ascii::new("Upgrade"),
        ];
    }

    HOP_HEADERS.iter().any(|h| h == &name)
}

/// Returns a clone of the headers without the [hop-by-hop headers].
///
/// [hop-by-hop headers]: http://www.w3.org/Protocols/rfc2616/rfc2616-sec13.html
fn remove_hop_headers(headers: &HeaderMap<HeaderValue>) -> HeaderMap<HeaderValue> {
    let mut result = HeaderMap::new();
    for (k, v) in headers.iter() {
        if !is_hop_header(k.as_str()) {
            result.insert(k.clone(), v.clone());
        }
    }
    result
}

pub fn create_proxied_request<B>(
    client_ip: IpAddr,
    backend: &config::Backend,
    mut request: Request<B>,
) -> Result<Request<B>, hyper::Error> {
    let path_and_query = request
        .uri()
        .path_and_query()
        .map(|x| x.as_str())
        .unwrap_or("/")
        .split('/')
        .skip(2)
        .fold(String::new(), |a, b| a + "/" + b);

    let uri = format!("{}{}", backend.url, path_and_query);
    let uri: Uri = uri.parse().unwrap();
    let host = get_host_from_uri(&uri);
    *request.uri_mut() = uri;

    request
        .headers_mut()
        .insert(HOST, HeaderValue::from_str(&host).unwrap());

    *request.headers_mut() = remove_hop_headers(request.headers());

    // Add forwarding information in the headers
    match request.headers_mut().entry("x-forwarded-for") {
        hyper::header::Entry::Vacant(entry) => {
            entry.insert(client_ip.to_string().parse().unwrap());
        }

        hyper::header::Entry::Occupied(mut entry) => {
            let addr = format!("{}, {}", entry.get().to_str().unwrap(), client_ip);
            entry.insert(addr.parse().unwrap());
        }
    }

    // Add Via header
    request
        .headers_mut()
        .insert(VIA, HeaderValue::from_static(SERVER_VIA));

    Ok(request)
}

pub fn request_add_custom_headers<B>(
    backend: &config::Backend,
    mut request: Request<B>,
) -> Request<B> {
    if let Some(headers) = &backend.headers {
        for (name, value) in headers.iter() {
            request.headers_mut().insert(
                HeaderName::from_lowercase(name.to_lowercase().as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
    }
    request
}

pub fn create_proxied_response<B>(mut response: Response<B>) -> Response<B> {
    *response.headers_mut() = remove_hop_headers(response.headers());
    response
        .headers_mut()
        .insert(VIA, HeaderValue::from_static(SERVER_VIA));
    response
}

pub fn get_host_from_uri(uri: &Uri) -> String {
    uri.authority().unwrap().host().to_string()
}

#[cfg(test)]
mod tests {

    use super::get_host_from_uri;
    use hyper::Uri;
    use std::str::FromStr;

    fn test_uri_host(uri: &str, host: &str) {
        let uri = Uri::from_str(uri).unwrap();
        assert_eq!(get_host_from_uri(&uri), host);
    }

    #[test]
    fn host_from_uri() {
        test_uri_host("http://example.com", "example.com");
        test_uri_host("https://example.com", "example.com");
    }

    #[test]
    fn host_from_uri_with_path() {
        test_uri_host("http://example.com/", "example.com");
        test_uri_host("http://example.com/path", "example.com");
        test_uri_host("http://example.com/path/to/foo", "example.com");
        test_uri_host("http://example.com/path/to/foo/", "example.com");
        test_uri_host("https://example.com/", "example.com");
        test_uri_host("https://example.com/path", "example.com");
        test_uri_host("https://example.com/path/to/foo", "example.com");
        test_uri_host("https://example.com/path/to/foo/", "example.com");
    }

    #[test]
    fn host_from_uri_with_query() {
        test_uri_host("http://example.com?bees=true", "example.com");
        test_uri_host("http://example.com/?bees=true", "example.com");
        test_uri_host("https://example.com?bees=true", "example.com");
        test_uri_host("https://example.com/?bees=true", "example.com");
    }

    #[test]
    fn host_from_uri_with_path_and_query() {
        test_uri_host("http://example.com/foo/?bees=true", "example.com");
        test_uri_host("http://example.com/foo/bar/?bees=true", "example.com");
        test_uri_host("https://example.com/foo/?bees=true", "example.com");
        test_uri_host("https://example.com/foo/bar/?bees=true", "example.com");
        test_uri_host("http://example.com/foo?bees=true", "example.com");
        test_uri_host("http://example.com/foo/bar?bees=true", "example.com");
        test_uri_host("https://example.com/foo?bees=true", "example.com");
        test_uri_host("https://example.com/foo/bar?bees=true", "example.com");
    }
}
