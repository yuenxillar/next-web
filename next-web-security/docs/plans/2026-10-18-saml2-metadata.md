# SAML 2.0 Metadata Slice — Execution Record

**Goal:** publish the service provider metadata an asserting party is configured from, without touching the login or logout profiles.

**Scope chosen by the caller:** metadata endpoint plus its configuration surface. Login (AuthnRequest, assertion consumption) and single logout stay out; the registration and repository are shaped so those slices can build on them.

**Reference:** `D:\EdgeDownload\spring-security-main` — `Saml2MetadataConfigurer.java`, `Saml2MetadataFilter.java`, `web/metadata/RequestMatcherMetadataResponseResolver.java`, `BaseOpenSamlMetadataResolver.java`, and the tests `Saml2MetadataFilterTests`, `Saml2MetadataConfigurerTests`, `OpenSaml5MetadataResolverTests`, `RequestMatcherMetadataResponseResolverTests`.

## What was built

| File | Responsibility |
|---|---|
| `src/spring_security/saml2/mod.rs` | Module entry; names the implemented slice |
| `src/spring_security/saml2/registration/relying_party_registration.rs` | `RelyingPartyRegistration` + builder: registration id, entity id template, ACS location/binding, optional SLO location/response location/binding, optional name id format, optional signing/encryption certificates |
| `src/spring_security/saml2/registration/relying_party_registration_repository.rs` | `RelyingPartyRegistrationRepository` trait (`find_by_registration_id`, `iter`) and the in-memory implementation |
| `src/spring_security/saml2/registration/saml2_message_binding.rs` | `Saml2MessageBinding` (`Post`/`Redirect`) with the metadata URN |
| `src/spring_security/saml2/metadata/saml2_metadata.rs` | The document writer: `EntityDescriptor` / `SPSSODescriptor`, placeholder resolution, XML escaping, certificate armor stripping |
| `src/spring_security/web/saml2/metadata/saml2_metadata_filter.rs` | The endpoint: three default patterns, single/all responses, 401 for an unknown id, response headers, base URL per request |
| `src/spring_security/config/web/configurers/saml2_metadata_configurer.rs` | Wiring: repository from DSL → shared object → context, optional `metadataUrl`, filter inserted before `BasicAuthenticationFilter` |

## Contract implemented (from the reference)

- Endpoints, any method: `/saml2/metadata/{registrationId}`, `/saml2/metadata` (all), `/saml2/service-provider-metadata/{registrationId}` (legacy alias). `metadataUrl(url)` replaces the matcher with that single pattern.
- Unknown registration id → **401**, empty body, no headers, chain not run. Unmatched request → passed down the chain.
- Success → 200 with `Content-Type: application/samlmetadata+xml;charset=UTF-8`, `Content-Disposition: attachment; filename="saml-<id>-metadata.xml"; filename*=UTF-8''<encoded>`, `Content-Length`, UTF-8 body.
- Document: `<md:EntityDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata" entityID="…">` with `<md:SPSSODescriptor protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">`, children in schema order — `KeyDescriptor` (signing, then encryption), `SingleLogoutService`, `NameIDFormat`, `AssertionConsumerService index="1"`. Several registrations are wrapped in `<md:EntitiesDescriptor>`; exactly one is a bare `EntityDescriptor`.
- No `AuthnRequestsSigned`, `WantAssertionsSigned`, `isDefault`, `ID`, `validUntil`, `cacheDuration` — the reference does not emit them either.
- `{baseUrl}` and `{registrationId}` are resolved when the document is rendered, from the request (scheme, host, non-default port, context path).
- Defaults: entity id `{baseUrl}/saml2/service-provider-metadata/{registrationId}`, ACS location `{baseUrl}/login/saml2/sso/{registrationId}`, ACS/SLO binding `HTTP-POST`, SLO location absent, SLO response location defaults to the SLO location.

## Tests

21 new, all seen failing before the code existed. Full crate suite: **193 passed, 0 failed**.

- Registration/repository (13): defaults, builder overrides, missing SLO, response location defaulting, bindings, name id format absent by default, empty id rejected, lookup/unknown/iteration order.
- Document writer (11): entity id and ACS, placeholder resolution, SLO omitted without a location, SLO with binding and response location, redirect binding, certificate armor stripped to one line, no `KeyDescriptor` without credentials, name id format text, single vs `EntitiesDescriptor`, attribute escaping.
- Filter (6): both URL shapes, all-registrations response, 401 for unknown id, chain passthrough, filename validation.
- Configurer (4): filter registered, missing repository reported, custom URL replaces the endpoints, repository taken from the context.

## Rulings

- **No `quick-xml`.** The document is written as text with one escaping helper. Cost if wrong: no schema validation, and escaping is our responsibility (covered by `attribute_values_are_escaped`).
- **No `Saml2MetadataResolver` trait and no custom response resolver.** The filter owns one writer. Cost if wrong: an application that wants a rewritten document must change the filter, unlike the reference's `metadataResponseResolver(...)`.
- **Signed metadata is not implemented** (the reference defaults it off). Cost if wrong: an asserting party that requires a signed metadata document cannot be served yet.
- **The all-registrations file name uses a random 16-hex id** instead of the reference's UUID, because the crate has no UUID dependency. Cost if wrong: a different (still non-identifying) file name.
- **The filename is URL-encoded once**, like the reference's legacy filter path, not twice like its production resolver. Cost if wrong: for a non-ASCII registration id the `filename*` value differs from the reference's (the reference double-encodes).
- **`metadataUrl` takes `self` by value** (it keeps the existing stub's builder shape; the other configurers take `&mut self`). Cost if wrong: one inconsistent DSL signature.
- **The filter is inserted with `add_filter_before::<_, BasicAuthenticationFilter>`** because no order is registered for it; this matches the reference's insertion point without touching `FilterOrderRegistration`. Cost if wrong: the filter's position is tied to the basic authentication filter's registration.
- **The repository is resolved by type from the context** (`get_singletons_by_type`), since `get_singleton_option` looks a value up by its default name. Cost if wrong: an application that registers the repository under an unusual type alias is not found.

## Not in this slice

- Login: `Saml2WebSsoAuthenticationFilter`, AuthnRequest handling, assertion verification/decryption, `AssertingPartyMetadata` beyond what metadata needs.
- Logout endpoints (`Saml2LogoutConfigurer` and its filters) — only the SLO advertisement is emitted.
- `Saml2LoginConfigurer`-sourced repository lookup (the reference's first choice); the port resolves DSL → shared object → context.
- Metadata signing, `EntityDescriptorCustomizer`.
