// The days left to a due date (src/lib/due.ts).

import { strict as assert } from "node:assert";
import { test } from "node:test";
import { daysLeft, formatDate, parseDate, shortDate, urgency } from "../src/lib/due.ts";

const now = Date.parse("2026-11-10T00:00:00Z");

test("the days from now to the date, negative once past", () => {
  assert.equal(daysLeft("2026-11-14", now), 4);
  assert.equal(daysLeft("2026-11-10", now), 0);
  assert.equal(daysLeft("2026-11-07", now), -3);
});

test("a date written in each format, and read back", () => {
  assert.equal(formatDate("2026-11-04", "ymd"), "2026-11-04");
  assert.equal(formatDate("2026-11-04", "dmy"), "04.11.2026");
  assert.equal(formatDate("2026-11-04", "mdy"), "11/04/2026");
  for (const format of ["ymd", "dmy", "mdy"] as const) {
    assert.equal(parseDate(formatDate("2026-11-04", format), format), "2026-11-04");
  }
});

test("a date typed loosely, or not a date", () => {
  assert.equal(parseDate("4.11.2026", "dmy"), "2026-11-04");
  assert.equal(parseDate("4 11 2026", "dmy"), "2026-11-04");
  assert.equal(parseDate("2026/1/9", "ymd"), "2026-01-09");
  assert.equal(parseDate("31.02.2026", "dmy"), null);
  assert.equal(parseDate("14.11.26", "dmy"), null);
  assert.equal(parseDate("soon", "ymd"), null);
});

test("a date for a tile, and how near it is", () => {
  assert.equal(shortDate("2026-11-14"), "14 Nov");
  assert.deepEqual([10, 7, 3, 1, 0, -2].map(urgency), ["far", "near", "close", "close", "due", "due"]);
});
