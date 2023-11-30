## Run the server

Create a cert with `certbot` and copy `fullchain.pem` and `privkey.pem` into this directory.

Change the candidate URL in `server/src/main.rs` to the correct URL (ex. `https://example.com:14192/`)

```
docker build . -t webrtc_proxy_server
docker run -d --rm --net host webrtc_proxy_server
```
