# vestirust

!!! WORK IN PROGRESS !!! Rust version of Vestibule

## TODO

- [ ] Reverse proxy unit tests
- [ ] Feat: static html

- [/] User authentication and security (local accounts)
  => Argon2 password hash
  => cookie lifetime
- [ ] User authentication and security (OAuth2)
- [ ] Frontend

- [ ] Use research

- [x] Graceful shutdown
- [ ] Log levels

- [/] Webdav file server
- [ ] Use X-OC-Mtime header to alter modtime on PUT requests (nextcloud/owncloud style)

- [ ] Security : inject security headers (CSP, etc)
- [ ] Security : CSRF protection
- [ ] Security : harden cookie (HTTP Only, Secure, etc.)

- [ ] Error handling
- [ ] Remove clones, panics, expects, unwraps, println!, etc.
- [ ] Tests
- [ ] Lifetimes for non serialized structs
- [ ] Litmus compliance in CI tests
- [ ] Remove axum macros
