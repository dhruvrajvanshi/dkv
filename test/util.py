from redis import Redis
import functools
import pytest
import os


def make_redis(protocol):
    r = Redis(
        host="localhost",
        port=int(os.environ.get("DKV_PORT", "6543")),
        protocol=protocol,
        decode_responses=True,
    )
    r.flushall()
    return r


def with_supported_protocols(f):
    @functools.wraps(f)
    @pytest.mark.parametrize("protocol", [2, 3])
    def wrapper(protocol, *args, **kwargs):
        return f(protocol, *args, **kwargs)

    return wrapper
