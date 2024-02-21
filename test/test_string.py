from test.util import make_redis, with_supported_protocols


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
