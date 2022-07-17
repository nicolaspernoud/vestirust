# vestirust

!!! WORK IN PROGRESS !!! Rust version of Vestibule

## TODO

- [ ] Fix: http2 downgrade
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
- [ ] Check that trying to PUT/MOVE/COPY on existing file/dir gives an error
- [ ] Fix copying dir on existing dir

- [ ] Error handling
- [ ] Remove clones, panics, expects, unwraps, println!, etc.
- [ ] Tests
- [ ] Litmus compliance in CI tests
