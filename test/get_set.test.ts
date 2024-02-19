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

test("get for non existent key", async () => {
  redis.set("foo", "bar")
  const value = await redis.get("foo")
  expect(value).toEqual("bar")
})
