import { afterEach, beforeEach, describe, expect, test } from "vitest"
import { createClient } from "redis"

describe("HASH", async () => {
  let redis: ReturnType<typeof createClient>

  beforeEach(async () => {
    redis = createClient({
      url: "redis://0.0.0.0:6543",
    })
    await redis.connect()
  })
  afterEach(async () => {
    await redis.disconnect()
  })
  test("HSET and HGET", async () => {
    await redis.hSet("myhash1", "field1", "Hello")
    await redis.hSet("myhash1", "field2", "World")
    expect(await redis.hGet("myhash1", "field1")).toEqual("Hello")
    expect(await redis.hGet("myhash1", "field2")).toEqual("World")
  })

  test("HSET converts numeric hash field to string", async () => {
    await redis.hSet("myhash", 1, "hello")
    expect(await redis.hGet("myhash", "1")).toEqual("hello")
  })
})
