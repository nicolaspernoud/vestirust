# vestirust

!!! WORK IN PROGRESS !!! Rust version of Vestibule

## TODO

[x] Mock of proxied services

[/] Reverse proxy configuration and dynamic loading
[ ] Fix: http2 downgrade
[ ] Feat: static html
[/] Performance: Configuration out of Mutex and full reloading
[/] Performance: do not extract twice the apps/davs (one on main router, one on proxy handler), but add the found app to request context

[ ] Let's encrypt certificates with acme-lib
[/] Webdav file server
[ ] Error handling
[ ] Remove clones, panics and unwraps
[ ] Webdav tests : unitary and integration
[ ] Derive key from passphrase and pass it along
[ ] Create a webdav server for each webdav in configuration and put it into a (the ?) hashtab
[ ] Use better maintained dav-server over dav-handler ?
[ ] Webdav file server encryption
[ ] Webdav file server zip folder
[ ] User authentication and security (local accounts)
[ ] User authentication and security (OAuth2)
[ ] Frontend
[ ] Tests
