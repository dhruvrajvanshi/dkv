import { beforeAll, beforeEach, expect, test } from "vitest"
import { createClient } from "redis"

let redis: Awaited<ReturnType<typeof createClient>>
beforeAll(async () => {
  redis = createClient({
    url: "redis://0.0.0.0:6543",
  })
  await redis.connect()
})
beforeEach(async () => {
  await redis.flushAll()
})

test("get should return a value after set", async () => {
  await redis.set("foo", "bar")
  const value = await redis.get("foo")
  expect(value).toEqual("bar")
})

test("rename works", async () => {
  await redis.set("foo", "bar")
  await redis.rename("foo", "baz")
  const value = await redis.get("baz")
  expect(value).toEqual("bar")
})

test("Rename of non existent key returns an error", async () => {
  try {
    await redis.rename("foo", "baz")
  } catch (e) {
    expect(e.message).toEqual("ERROR: NO_SUCH_KEY")
  }
})
