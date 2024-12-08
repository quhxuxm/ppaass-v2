#!/bin/sh
a=0
while [ $a -lt 5 ];
do
    process_id=$(ps -ef | grep "ppaass-v2-proxy" | grep -v grep | awk '{print $2}')
    if [ -z "$process_id"]; then
        echo "No ppaass-v2-proxy process"
    else
        echo "Found ppaass-v2-proxy process: $process_id"
        kill -9 $process_id
    fi
    a=`expr $a + 1`
    sleep 2
done

ulimit -n 409600
RUST_BACKTRACE=1 ./ppaass-v2-proxy -c resources/config.toml
