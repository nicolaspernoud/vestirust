# vestirust

!!! WORK IN PROGRESS !!! Rust version of Vestibule

## TODO

- [x] Mock of proxied services

- [x] Reverse proxy configuration and dynamic loading
- [ ] Fix: http2 downgrade
- [ ] Feat: static html
- [x] Performance: Configuration out of Mutex and full reloading
- [/] Performance: do not extract twice the apps/davs (one on main router, one on proxy handler), but add the found app to request context

- [ ] Let's encrypt certificates with acme-lib
- [/] Webdav file server

- [ ] Webdav tests : unitary and integration
- [ ] Derive key from passphrase and pass it along
- [x] Webdav file server encryption
- [x] Webdav file server zip folder
- [ ] User authentication and security (local accounts)
- [ ] User authentication and security (OAuth2)
- [ ] Frontend
- [ ] Test that decrypting with the wrong key fails
- [ ] Refactor the oassphrase test on dav server

- [ ] Use research

- [x] Graceful shutdown
- [ ] Log levels

- [x] Test that is not possible to get à ".." directory
- [ ] Error handling
- [ ] Remove clones, panics, expects, unwraps, println!, etc.
- [ ] Tests

- [ ] Use X-OC-Mtime header to alter modtime on PUT requests (nextcloud/owncloud style)
- [ ] Check that trying to PUT/MOVE/COPY on existing file/dir gives an error
