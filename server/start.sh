set -m
cargo run --release &
sleep 3
/etc/init.d/nginx start
fg
