# Changelog
All notable changes to this project will be documented in this file.

## 0.2.3

### Changed

- Requires `next-web-state-machine` 0.2.0, which adds the service layer, the
  `#[on_transition]` handler attribute, restoration of the extended state and the
  serialization of the events of one machine. See the changelog of that crate for
  the migration notes.
- Re-exports `on_transition` under the `enable-state-machine` feature, so an
  application which enables the feature writes
  `#[next_web::on_transition(..)]` and needs no extra dependency.
