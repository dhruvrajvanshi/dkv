import { createClient } from "redis"
import { afterAll, beforeAll, beforeEach } from "vitest"

export const redis = createClient({
  url: "redis://0.0.0.0:6543",
})

beforeAll(async () => {
  await redis.connect()
})
beforeEach(async () => {
  await redis.flushAll()
})
afterAll(async () => {
  await redis.disconnect()
})
