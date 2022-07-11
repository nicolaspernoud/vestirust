# vestirust

!!! WORK IN PROGRESS !!! Rust version of Vestibule

## TODO

- [ ] Fix: http2 downgrade
- [ ] Feat: static html
- [/] Performance: do not extract twice the apps/davs (one on main router, one on proxy handler), but add the found app to request context

- [/] Webdav file server

- [ ] User authentication and security (local accounts)
- [ ] User authentication and security (OAuth2)
- [ ] Frontend

- [ ] Use research

- [x] Graceful shutdown
- [ ] Log levels

- [ ] Use X-OC-Mtime header to alter modtime on PUT requests (nextcloud/owncloud style)
- [ ] Check that trying to PUT/MOVE/COPY on existing file/dir gives an error
- [ ] Fix copying dir on existing dir

- [ ] Error handling
- [ ] Remove clones, panics, expects, unwraps, println!, etc.
- [ ] Tests
