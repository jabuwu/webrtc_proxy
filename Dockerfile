FROM rust
WORKDIR /webrtc_proxy
RUN apt-get update
RUN apt-get install -y nginx libnginx-mod-stream vim
COPY server/nginx/nginx.conf /etc/nginx/nginx.conf
COPY server/nginx/default /etc/nginx/sites-available/default
COPY server server/
COPY enaia_server enaia_server/
RUN (cd server && cargo build --release)
WORKDIR /webrtc_proxy/server
COPY fullchain.pem .
COPY privkey.pem .
RUN chmod +x start.sh
CMD ["bash", "./start.sh"]
