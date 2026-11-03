// The days left to a due date (src/lib/due.ts).

import { strict as assert } from "node:assert";
import { test } from "node:test";
import { daysLeft } from "../src/lib/due.ts";

const now = Date.parse("2026-11-10T00:00:00Z");

test("the days from now to the date, negative once past", () => {
  assert.equal(daysLeft("2026-11-14", now), 4);
  assert.equal(daysLeft("2026-11-10", now), 0);
  assert.equal(daysLeft("2026-11-07", now), -3);
});
