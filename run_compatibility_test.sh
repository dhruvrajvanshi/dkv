#!/bin/bash
set -e

if [ -d "compatibility-test-suite-for-redis" ]; then
  echo "Compatibility test suite already exists..."
else
  echo "Cloning compatibility test suite..."
  git clone https://github.com/tair-opensource/compatibility-test-suite-for-redis.git
fi

echo "Installing dependencies"
pip install -r compatibility-test-suite-for-redis/requirements.txt

cargo run --bin dkv &
child_pid=$!


echo -n 'Waiting for port 6543 to open...'
until nc -z 0.0.0.0 6543; do
  sleep 1
done
echo 'Port 6543 is now open!'


python compatibility-test-suite-for-redis/redis_compatibility_test.py --port 6543 --testfile compatibility-test-suite-for-redis/cts.json > test_result.log
cat test_result.log
cat test_result.log | python parse_test_log.py

cat test_result.log | python parse_test_log.py >> $GITHUB_STEP_SUMMARY


trap "kill $child_pid" EXIT
