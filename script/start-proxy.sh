ulimit -n 409600
sudo ps -ef | grep ppaass-v2-proxy | grep -v grep | awk '{print $2}' | xargs sudo kill -9
RUST_BACKTRACE=1 ./ppaass-v2-proxy -c resources/config.toml