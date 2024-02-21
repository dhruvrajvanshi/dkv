from test.util import make_redis, with_supported_protocols


@with_supported_protocols
def test_hget_with_nonexistent_key(protocol):
    redis = make_redis(protocol)
    redis.hset("hash", "field", "value")
    assert redis.hget("hash", "field2") is None


@with_supported_protocols
def test_hget_after_hset(protocol):
    redis = make_redis(protocol)
    redis.hset("myhash1", "field1", "Hello")
    redis.hset("myhash1", "field2", "World")
    assert redis.hget("myhash1", "field1") == "Hello"
    assert redis.hget("myhash1", "field2") == "World"
