# Spring Security Translation Coverage

Java baseline: `D:\EdgeDownload\spring-security-main\spring-security-main`

## Implemented mainline

- `access`
- `authentication`
- `authorization`
- `config`
- `core`
- `crypto`
- `permission`
- `web`

These modules now compile in Rust and the Phase 1 builder chain is wired far enough to pass:

```text
cargo test -p next-web-security
```

## Added translation entry points

- `acl`
- `cas`
- `data`
- `kerberos`
- `messaging`
- `oauth2`
- `rsocket`
- `saml2`
- `webauthn`

These modules are now present in `src/spring_security` so the Rust tree matches the Java top-level module shape more closely, but they are not yet feature-complete translations.

## LDAP progress

The `ldap` module now includes a first usable Rust translation for:

- directory context model (`DirContextOperations`)
- LDAP user search trait (`LdapUserSearch`)
- LDAP authorities population trait (`LdapAuthoritiesPopulator`)
- LDAP user details traits and default implementation
- user-details context mapping
- `LdapUserDetailsService`
- `LdapAuthenticator`
- `LdapAuthenticationProvider`

Current gaps versus Java:

- no bind/password-comparison authenticator implementations yet
- no default group-search based authorities populator
- no Active Directory provider
- no password-policy, embedded server, or Jackson support yet

## Authentication progress

The `authentication` and supporting `core/userdetails` translation now include:

- `AccountStatusUserDetailsChecker`
- status-oriented authentication error mappings for bad credentials, locked, disabled, account-expired, and credentials-expired cases
- `AbstractUserDetailsAuthenticationProvider`-style support state for cache, pre/post checks, authorities mapping, and success-token creation
- `DaoAuthenticationProvider`

Current gaps versus Java:

- no `CompromisedPasswordChecker`
- no delegating/default password encoder factory equivalent yet
- `AuthenticationManagerBuilder` is still much simpler than Spring Security's provider-manager assembly pipeline
- success principal is currently normalized to username string rather than exposing the full Java-style principal selection matrix

## Remaining parity work

- Align current Rust `config/web` DSL behavior more closely with Spring Security Java semantics.
- Translate enterprise and protocol modules package-by-package instead of only adding top-level placeholders.
- Add module-specific tests for authentication flows, request matching, ACL semantics, LDAP integrations, OAuth2 token flows, SAML2 flows, CAS, Kerberos, WebAuthn, RSocket, and messaging authorization.
