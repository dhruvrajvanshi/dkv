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


@with_supported_protocols
def test_hgetall(protocol):
    redis = make_redis(protocol)
    redis.hset("myhash2", "field1", "Hello")
    redis.hset("myhash2", "field2", "World")
    assert redis.hgetall("myhash2") == {"field1": "Hello", "field2": "World"}


@with_supported_protocols
def test_hgetall_non_existent_hash(protocol):
    redis = make_redis(protocol)
    assert redis.hgetall("nonexistent") == {}
