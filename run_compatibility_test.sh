#!/bin/sh
set -e

if [ -d "venv" ]; then
  echo "Virtual env already exists..."
else
  echo "Creating virtual env..."
  python -m venv venv
fi

source venv/Scripts/activate

if [ -d "compatibility-test-suite-for-redis" ]; then
  echo "Compatibility test suite already exists..."
else
  echo "Cloning compatibility test suite..."
  git clone git@github.com:tair-opensource/compatibility-test-suite-for-redis.git compatibility-test-suite-for-redis
fi

echo "Installing dependencies"
pip install -r compatibility-test-suite-for-redis/requirements.txt

cargo run --bin dkv &
child_pid=$!

python compatibility-test-suite-for-redis/redis_compatibility_test.py --port 6543 --testfile compatibility-test-suite-for-redis/tests/cts.json

source venv/Scripts/deactivate
kill $child_pid
