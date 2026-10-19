//! The XML signature of a SAML 2.0 message.
//!
//! A SAML 2.0 response is believed only when the signature an asserting party put
//! on it covers what the response claims: the digest of the signed element has to
//! match the one the signature carries, and the signature itself has to verify
//! against the key the registration holds.
//!
//! A signature is defined over the *canonical* form of a document (Exclusive XML
//! Canonicalization), so this module parses the document into a tree,
//! canonicalizes the element a reference names, and verifies the two values. Only
//! the algorithms of a SAML 2.0 profile are accepted — RSA-SHA256, SHA-256 and the
//! enveloped-signature transform — and anything else is refused rather than
//! guessed at.
//!
//! A document whose form differs from its canonical form cannot be verified this
//! way and is refused: rewriting it would produce a digest that is not the digest
//! the asserting party signed. A SAML 2.0 profile requires the signature to be over
//! the canonical form, which is what makes this check the right one.

use std::collections::BTreeMap;

use base64::{Engine, engine::general_purpose::STANDARD};
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use rsa::{
    RsaPublicKey, pkcs1::DecodeRsaPublicKey, pkcs1v15::VerifyingKey, pkcs8::DecodePublicKey,
    signature::Verifier,
};
use sha2::{Digest, Sha256};

use crate::saml2::error::{Saml2Error, Saml2ErrorKind};

/// The namespace of XML signatures.
const DSIG_NAMESPACE: &str = "http://www.w3.org/2000/09/xmldsig#";

/// The canonicalization a signature this module accepts was taken with.
const EXCLUSIVE_C14N: &str = "http://www.w3.org/2001/10/xml-exc-c14n#";

/// The transform that removes the signature from the element it signs.
const ENVELOPED_SIGNATURE: &str = "http://www.w3.org/2000/09/xmldsig#enveloped-signature";

/// The signature algorithm of a response.
const RSA_SHA256: &str = "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256";

/// The digest algorithm of a reference.
const SHA256: &str = "http://www.w3.org/2001/04/xmlenc#sha256";

/// Verifies the signature of a document against a public key.
///
/// # Errors
///
/// Returns [`Saml2ErrorKind::VerificationFailed`] when the document carries no
/// signature, when the signature names an algorithm this profile does not use,
/// when a digest does not match the element it covers, or when the signature does
/// not verify against the key.
pub(crate) fn verify_signature(document: &str, public_key_pem: &str) -> Result<(), Saml2Error> {
    let root = parse_document(document)?;
    let mut found = Vec::new();
    collect_signatures(&root, &mut found);
    let signature = found
        .first()
        .ok_or_else(|| failed("the message carries no signature"))?;

    verify_one(signature, &root, public_key_pem)
}

/// Returns the error of a message that is not believed.
fn failed(reason: impl Into<String>) -> Saml2Error {
    Saml2Error::new(Saml2ErrorKind::VerificationFailed, reason.into())
}

/// Verifies one signature of a document.
fn verify_one(
    signature: &Element,
    root: &Element,
    public_key_pem: &str,
) -> Result<(), Saml2Error> {
    let signed_info = signature
        .child("SignedInfo")
        .ok_or_else(|| failed("the signature carries no SignedInfo"))?;

    let canonicalization = signed_info
        .child("CanonicalizationMethod")
        .and_then(|method| method.attribute("Algorithm"))
        .ok_or_else(|| failed("the signature names no canonicalization"))?;
    if canonicalization != EXCLUSIVE_C14N {
        return Err(failed(format!(
            "the canonicalization {canonicalization} is not one this profile uses"
        )));
    }

    let signature_method = signed_info
        .child("SignatureMethod")
        .and_then(|method| method.attribute("Algorithm"))
        .ok_or_else(|| failed("the signature names no algorithm"))?;
    if signature_method != RSA_SHA256 {
        return Err(failed(format!(
            "the signature algorithm {signature_method} is not one this profile uses"
        )));
    }

    let signature_value = signature
        .child("SignatureValue")
        .map(|value| value.text.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| failed("the signature carries no value"))?;

    // Every reference of the signature has to hold: a signature over a part of a
    // document proves nothing about the rest of it.
    let references = signed_info.children_named("Reference");
    if references.is_empty() {
        return Err(failed("the signature covers no element"));
    }
    for reference in references {
        verify_reference(reference, root)?;
    }

    // The signature is over the canonical form of `SignedInfo`, with the
    // namespaces that are in scope where the signature sits.
    let scope = in_scope_of(root, signature).unwrap_or_default();
    let canonical_signed_info = canonicalize(signed_info, None, &scope);
    let signature_bytes = STANDARD
        .decode(signature_value.as_bytes())
        .map_err(|error| failed(format!("the signature is not base64: {error}")))?;
    let key = parse_public_key(public_key_pem)?;
    let verifying_key = VerifyingKey::<Sha256>::new(key);
    let signature = rsa::pkcs1v15::Signature::try_from(signature_bytes.as_slice())
        .map_err(|_| failed("the signature is not an RSA signature"))?;
    verifying_key
        .verify(canonical_signed_info.as_bytes(), &signature)
        .map_err(|_| {
            failed("the signature does not verify against the key of the asserting party")
        })?;

    Ok(())
}

/// Verifies one reference of a signature.
fn verify_reference(reference: &Element, root: &Element) -> Result<(), Saml2Error> {
    let uri = reference
        .attribute("URI")
        .ok_or_else(|| failed("a reference of the signature covers no element"))?;
    let id = uri
        .strip_prefix('#')
        .ok_or_else(|| failed(format!("the reference {uri} is not an element of this document")))?;

    let digest_method = reference
        .child("DigestMethod")
        .and_then(|method| method.attribute("Algorithm"))
        .ok_or_else(|| failed("a reference of the signature names no digest"))?;
    if digest_method != SHA256 {
        return Err(failed(format!(
            "the digest {digest_method} is not one this profile uses"
        )));
    }
    let digest_value = reference
        .child("DigestValue")
        .map(|value| value.text.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| failed("a reference of the signature carries no digest"))?;

    // The enveloped-signature transform is what a SAML 2.0 profile uses for the
    // signature of the element it sits in; a transform this module does not know
    // would have to be applied to verify the digest, so it is refused.
    if let Some(transforms) = reference.child("Transforms") {
        for transform in transforms.children_named("Transform") {
            match transform.attribute("Algorithm").as_deref() {
                Some(ENVELOPED_SIGNATURE) | Some(EXCLUSIVE_C14N) => {}
                Some(other) => {
                    return Err(failed(format!(
                        "the transform {other} is not one this profile uses"
                    )));
                }
                None => return Err(failed("a transform of the signature names no algorithm")),
            }
        }
    }

    let signed = root
        .find_by_id(id)
        .ok_or_else(|| failed(format!("the document holds no element with the id {id}")))?;
    let signature = enclosing_signature(signed);
    let scope = in_scope_of(root, signed).unwrap_or_default();
    let canonical = canonicalize(signed, signature, &scope);
    let digest = Sha256::digest(canonical.as_bytes());
    let expected = STANDARD
        .decode(digest_value.as_bytes())
        .map_err(|error| failed(format!("the digest is not base64: {error}")))?;
    if digest.as_slice() != expected.as_slice() {
        return Err(failed(format!(
            "the element {id} is not the one the signature covers"
        )));
    }

    Ok(())
}

/// Returns the signature of an element, which the enveloped-signature transform
/// leaves out of the canonical form.
fn enclosing_signature(element: &Element) -> Option<&Element> {
    element.children.iter().find(|child| child.is("Signature"))
}

/// Parses a public key in PEM form.
fn parse_public_key(pem: &str) -> Result<RsaPublicKey, Saml2Error> {
    let pem = pem.trim();
    RsaPublicKey::from_public_key_pem(pem)
        .or_else(|_| RsaPublicKey::from_pkcs1_pem(pem))
        .map_err(|error| failed(format!("the key of the asserting party is not RSA PEM: {error}")))
}

/// An element of the parsed document.
#[derive(Clone, Debug, Default)]
struct Element {
    /// The prefix of the element name, empty when it has none.
    prefix: String,
    /// The local name of the element.
    local: String,
    /// The namespaces the element declares, as `(prefix, uri)`.
    namespaces: Vec<(String, String)>,
    /// The attributes of the element that are not namespace declarations, as
    /// `(prefix, local name, value)`.
    attributes: Vec<(String, String, String)>,
    /// The text the element holds.
    text: String,
    /// The elements the element holds.
    children: Vec<Element>,
}

impl Element {
    /// Returns whether the element is the one with the given local name in the
    /// signature namespace.
    fn is(&self, local: &str) -> bool {
        self.local == local && self.namespace_of(&self.prefix).as_deref() == Some(DSIG_NAMESPACE)
    }

    /// Returns the namespace the given prefix is bound to by this element.
    fn namespace_of(&self, prefix: &str) -> Option<String> {
        self.namespaces
            .iter()
            .find(|(declared, _)| declared == prefix)
            .map(|(_, uri)| uri.clone())
    }

    /// Returns the first child with the given local name.
    fn child(&self, local: &str) -> Option<&Element> {
        self.children.iter().find(|child| child.local == local)
    }

    /// Returns the children with the given local name.
    fn children_named(&self, local: &str) -> Vec<&Element> {
        self.children
            .iter()
            .filter(|child| child.local == local)
            .collect()
    }

    /// Returns the value of the attribute with the given local name, whatever
    /// its prefix.
    fn attribute(&self, local: &str) -> Option<String> {
        self.attributes
            .iter()
            .find(|(_, name, _)| name == local)
            .map(|(_, _, value)| value.clone())
    }

    /// Returns the element of the document with the given id.
    fn find_by_id(&self, id: &str) -> Option<&Element> {
        if self.attribute("ID").as_deref() == Some(id) {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find_by_id(id))
    }

    /// Returns the qualified name of the element.
    fn qualified_name(&self) -> String {
        if self.prefix.is_empty() {
            self.local.clone()
        } else {
            format!("{}:{}", self.prefix, self.local)
        }
    }
}

/// Collects the signatures of an element and of its children.
fn collect_signatures<'a>(element: &'a Element, found: &mut Vec<&'a Element>) {
    if element.is("Signature") {
        found.push(element);
    }
    for child in &element.children {
        collect_signatures(child, found);
    }
}

/// Returns the namespaces in scope where the given element sits.
fn in_scope_of(root: &Element, target: &Element) -> Option<BTreeMap<String, String>> {
    fn descend(
        element: &Element,
        target: &Element,
        scope: &mut BTreeMap<String, String>,
    ) -> bool {
        let before = scope.clone();
        for (prefix, uri) in &element.namespaces {
            scope.insert(prefix.clone(), uri.clone());
        }
        let found = std::ptr::eq(element, target)
            || element
                .children
                .iter()
                .any(|child| descend(child, target, scope));
        if !found {
            *scope = before;
        }

        found
    }

    let mut scope = BTreeMap::new();
    descend(root, target, &mut scope).then_some(scope)
}

/// Parses a document into the tree of its elements.
fn parse_document(document: &str) -> Result<Element, Saml2Error> {
    let mut reader = Reader::from_str(document);
    let mut stack: Vec<Element> = Vec::new();
    let mut root: Option<Element> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => stack.push(parse_element(&element)),
            Ok(Event::Empty(element)) => {
                let parsed = parse_element(&element);
                match stack.last_mut() {
                    Some(parent) => parent.children.push(parsed),
                    None => root = Some(parsed),
                }
            }
            Ok(Event::Text(text)) => {
                if let Some(current) = stack.last_mut() {
                    current.text.push_str(&text.unescape().unwrap_or_default());
                }
            }
            Ok(Event::CData(data)) => {
                if let Some(current) = stack.last_mut() {
                    current
                        .text
                        .push_str(&String::from_utf8_lossy(data.as_ref()));
                }
            }
            Ok(Event::End(_)) => {
                let element = stack
                    .pop()
                    .ok_or_else(|| failed("the document ends an element it never started"))?;
                match stack.last_mut() {
                    Some(parent) => parent.children.push(element),
                    None => root = Some(element),
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                return Err(Saml2Error::new(
                    Saml2ErrorKind::InvalidResponse,
                    format!("the document is not readable XML: {error}"),
                ));
            }
        }
    }

    root.ok_or_else(|| {
        Saml2Error::new(
            Saml2ErrorKind::InvalidResponse,
            "the document holds no element".to_string(),
        )
    })
}

/// Parses an element, its namespaces and its attributes.
fn parse_element(element: &BytesStart) -> Element {
    let name = element.name();
    let qualified = std::str::from_utf8(name.as_ref()).unwrap_or_default();
    let (prefix, local) = match qualified.split_once(':') {
        Some((prefix, local)) => (prefix.to_string(), local.to_string()),
        None => (String::new(), qualified.to_string()),
    };
    let mut namespaces = Vec::new();
    let mut attributes = Vec::new();
    for attribute in element.attributes().flatten() {
        let key = std::str::from_utf8(attribute.key.as_ref())
            .unwrap_or_default()
            .to_string();
        let value = attribute.unescape_value().unwrap_or_default().to_string();
        match key.strip_prefix("xmlns:") {
            Some(prefix) => namespaces.push((prefix.to_string(), value)),
            None if key == "xmlns" => namespaces.push((String::new(), value)),
            None => {
                let (prefix, local) = match key.split_once(':') {
                    Some((prefix, local)) => (prefix.to_string(), local.to_string()),
                    None => (String::new(), key.clone()),
                };
                attributes.push((prefix, local, value));
            }
        }
    }

    Element {
        prefix,
        local,
        namespaces,
        attributes,
        text: String::new(),
        children: Vec::new(),
    }
}

/// Returns the canonical form of an element.
///
/// The namespaces in scope where the element sits are given, and the signature
/// of the element is left out when one is given.
fn canonicalize(
    element: &Element,
    without: Option<&Element>,
    in_scope: &BTreeMap<String, String>,
) -> String {
    let mut output = String::new();
    let mut rendered = BTreeMap::new();
    write_element(
        element,
        without,
        in_scope,
        &mut rendered,
        &mut output,
    );

    output
}

/// Writes the canonical form of an element.
fn write_element(
    element: &Element,
    without: Option<&Element>,
    in_scope: &BTreeMap<String, String>,
    rendered: &mut BTreeMap<String, String>,
    output: &mut String,
) {
    let mut scope = in_scope.clone();
    for (prefix, uri) in &element.namespaces {
        scope.insert(prefix.clone(), uri.clone());
    }

    // Exclusive canonicalization writes only the namespaces the element or one of
    // its attributes visibly utilizes, and writes each of them once.
    let mut utilized = vec![element.prefix.clone()];
    for (prefix, _, _) in &element.attributes {
        utilized.push(prefix.clone());
    }
    utilized.sort();
    utilized.dedup();
    let mut declarations = Vec::new();
    for prefix in utilized {
        let Some(uri) = scope.get(&prefix) else {
            continue;
        };
        if rendered.get(&prefix) == Some(uri) {
            continue;
        }
        rendered.insert(prefix.clone(), uri.clone());
        declarations.push((prefix, uri.clone()));
    }
    declarations.sort_by(|(a, _), (b, _)| a.cmp(b));

    output.push('<');
    output.push_str(&element.qualified_name());
    for (prefix, uri) in &declarations {
        if prefix.is_empty() {
            output.push_str(" xmlns=\"");
        } else {
            output.push_str(&format!(" xmlns:{prefix}=\""));
        }
        output.push_str(&escape_attribute(uri));
        output.push('"');
    }
    let mut attributes = element.attributes.clone();
    attributes.sort_by(|(prefix_a, local_a, _), (prefix_b, local_b, _)| {
        let namespace_a = element.namespace_of(prefix_a).unwrap_or_default();
        let namespace_b = element.namespace_of(prefix_b).unwrap_or_default();
        namespace_a.cmp(&namespace_b).then_with(|| local_a.cmp(local_b))
    });
    for (prefix, local, value) in attributes {
        let name = if prefix.is_empty() {
            local
        } else {
            format!("{prefix}:{local}")
        };
        output.push_str(&format!(" {name}=\"{}\"", escape_attribute(&value)));
    }
    output.push('>');
    output.push_str(&escape_text(&element.text));
    for child in &element.children {
        if let Some(without) = without {
            if std::ptr::eq(child, without) {
                continue;
            }
        }
        write_element(child, without, &scope, rendered, output);
    }
    output.push_str(&format!("</{}>", element.qualified_name()));
}

/// Escapes the text of an element in the canonical form.
fn escape_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\r' => escaped.push_str("&#xD;"),
            other => escaped.push(other),
        }
    }

    escaped
}

/// Escapes an attribute value in the canonical form.
fn escape_attribute(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '"' => escaped.push_str("&quot;"),
            '\t' => escaped.push_str("&#x9;"),
            '\n' => escaped.push_str("&#xA;"),
            '\r' => escaped.push_str("&#xD;"),
            other => escaped.push(other),
        }
    }

    escaped
}
