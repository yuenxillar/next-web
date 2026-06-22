/// Encodes the username, realm, and password in A1 format for Digest authentication.
///
/// A1 = username + ":" + realm + ":" + password
/// Returns the MD5 hex of A1.
pub fn encode_password_in_a1_format(username: &str, realm: &str, password: &str) -> String {
    let a1 = format!("{}:{}:{}", username, realm, password);
    md5_hex(&a1)
}

/// Computes the MD5 hex digest of a string.
pub fn md5_hex(data: &str) -> String {
    let digest = md5::compute(data.as_bytes());
    format!("{:x}", digest)
}

/// Splits a string by a separator character, ignoring characters inside
/// double-quoted sections.
pub fn split_ignoring_quotes(str: &str, separator: char) -> Vec<String> {
    if str.is_empty() {
        return Vec::new();
    }

    let chars: Vec<char> = str.chars().collect();
    let len = chars.len();
    let mut result = Vec::new();
    let mut i = 0;
    let mut start = 0;
    let mut matched = false;

    while i < len {
        if chars[i] == '"' {
            // Skip the opening quote
            i += 1;
            // Skip everything until the closing quote
            while i < len {
                if chars[i] == '"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            matched = true;
            continue;
        }
        if chars[i] == separator {
            if matched {
                let segment: String = chars[start..i].iter().collect();
                result.push(segment);
                matched = false;
            }
            i += 1;
            start = i;
            continue;
        }
        matched = true;
        i += 1;
    }

    if matched {
        let segment: String = chars[start..i].iter().collect();
        result.push(segment);
    }

    result
}

/// Takes an array of strings, removes any characters in `remove_chars` from each
/// element, splits on `delimiter`, and creates a map where the left side is the key
/// and the right side is the value. Trims both key and value.
pub fn split_each_array_element_and_create_map(
    array: &[String],
    delimiter: &str,
    remove_chars: &str,
) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();

    for s in array {
        let post_remove = if !remove_chars.is_empty() {
            s.replace(remove_chars, "")
        } else {
            s.clone()
        };

        if let Some((key, value)) = post_remove.split_once(delimiter) {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    map
}

/// Computes the `response` portion of a Digest authentication header.
/// Both the server and user agent should compute the `response` independently.
///
/// # Parameters
/// * `password_already_encoded` - Whether the password is already in A1 format.
/// * `username`                 - The user's login name.
/// * `realm`                    - The realm name.
/// * `password`                 - The user's password (plaintext or A1-encoded).
/// * `http_method`              - The HTTP request method (GET, POST, etc.).
/// * `uri`                      - The request URI.
/// * `qop`                      - The qop directive, or None if not set.
/// * `nonce`                    - The nonce supplied by the server.
/// * `nc`                       - The nonce-count as defined in RFC 2617.
/// * `cnonce`                   - Opaque string supplied by the client when qop is set.
///
/// # Returns
/// The MD5 of the digest authentication response, encoded in hex.
pub fn generate_digest(
    password_already_encoded: bool,
    username: &str,
    realm: &str,
    password: &str,
    http_method: &str,
    uri: &str,
    qop: Option<&str>,
    nonce: &str,
    nc: &str,
    cnonce: &str,
) -> String {
    let a2 = format!("{}:{}", http_method, uri);
    let a2_md5 = md5_hex(&a2);

    let a1_md5 = if !password_already_encoded {
        encode_password_in_a1_format(username, realm, password)
    } else {
        password.to_string()
    };

    match qop {
        None => {
            // RFC 2069 compliant clients
            md5_hex(&format!("{}:{}:{}", a1_md5, nonce, a2_md5))
        }
        Some("auth") => {
            // RFC 2617 compliant clients
            md5_hex(&format!("{}:{}:{}:{}:{}:{}", a1_md5, nonce, nc, cnonce, "auth", a2_md5))
        }
        Some(other) => panic!("This method does not support a qop: '{}'", other),
    }
}
