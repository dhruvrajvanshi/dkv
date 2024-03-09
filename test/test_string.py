from test.util import make_redis, with_supported_protocols
import pytest
from redis.exceptions import ResponseError


@with_supported_protocols
def test_get_after_set(protocol):
    r = make_redis(protocol)
    r.set("foo", "bar")
    assert r.get("foo") == "bar"


@with_supported_protocols
def test_get_non_existent(protocol):
    r = make_redis(protocol)
    assert r.get("foo") is None


@with_supported_protocols
def test_rename(protocol):
    r = make_redis(protocol)
    r.set("foo", "bar")
    r.rename("foo", "bar")
    assert r.get("bar") == "bar"
    assert r.get("foo") is None


@with_supported_protocols
def test_exists(protocol):
    r = make_redis(protocol)
    r.set("foo", "bar")
    assert r.exists("foo") == 1
    assert r.exists("bar") == 0


@with_supported_protocols
def test_get_with_non_string_value(protocol):
    r = make_redis(protocol)
    r.hset("foo", "key", "value")
    with pytest.raises(ResponseError) as ex:
        r.get("foo")
    assert ex.match("WRONGTYPE")


@with_supported_protocols
def test_get_with_integer_value(protocol):
    r = make_redis(protocol)
    r.set("foo", 1)
    assert r.get("foo") == "1"

@with_supported_protocols
def test_del_non_existent(protocol):
    r = make_redis(protocol)
    r.delete("foo")

@with_supported_protocols
def test_del_existing(protocol):
    r = make_redis(protocol)
    r.set("foo", "bar")
    r.delete("foo")
    assert r.get("foo") is None

@with_supported_protocols
def test_del_multiple(protocol):
    r = make_redis(protocol)
    r.set("foo", "bar")
    r.set("bar", "baz")
    r.delete("foo", "bar")
    assert r.get("foo") is None
    assert r.get("bar") is None
