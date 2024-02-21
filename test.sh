#!/bin/sh
set -e

cargo run --bin dkv &
process_id=$!

echo -n 'Waiting for port 6543 to open...'
until nc -z 0.0.0.0 6543; do
  sleep 1
done
echo 'Port 6543 is now open!'

pytest --junit-xml=test-results.xml

trap "kill $process_id" EXIT
