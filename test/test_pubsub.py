from .util import make_redis, with_supported_protocols
from threading import Thread


@with_supported_protocols
def test_pubsub(protocol):
    r = make_redis(protocol)

    pubsub = r.pubsub()
    pubsub.subscribe("foo")

    def listener():
        count = 0
        for message in pubsub.listen():
            if message["type"] == "message":
                count += 1
                assert message["data"] == f"hello{count}"
            if count == 3:
                break

    t = Thread(target=listener)
    t.start()

    r.publish("foo", "hello1")
    r.publish("foo", "hello2")
    r.publish("foo", "hello3")

    t.join()
