const DAY = 86_400_000;
const pad = (n: number, w = 2): string => String(n).padStart(w, '0');

/** Milliseconds since the epoch to RFC 3339 UTC, by civil-calendar arithmetic. */
export function isoOf(ms: number): string {
  const days = Math.floor(ms / DAY);
  const rem = ms - days * DAY;
  const z = days + 719_468;
  const era = Math.floor(z / 146_097);
  const doe = z - era * 146_097;
  const yoe = Math.floor((doe - Math.floor(doe / 1460) + Math.floor(doe / 36_524) - Math.floor(doe / 146_096)) / 365);
  const doy = doe - (365 * yoe + Math.floor(yoe / 4) - Math.floor(yoe / 100));
  const mp = Math.floor((5 * doy + 2) / 153);
  const day = doy - Math.floor((153 * mp + 2) / 5) + 1;
  const month = mp < 10 ? mp + 3 : mp - 9;
  const year = yoe + era * 400 + (month <= 2 ? 1 : 0);
  const h = Math.floor(rem / 3_600_000);
  const mi = Math.floor((rem % 3_600_000) / 60_000);
  const sec = Math.floor((rem % 60_000) / 1000);
  return `${pad(year, 4)}-${pad(month)}-${pad(day)}T${pad(h)}:${pad(mi)}:${pad(sec)}.${pad(rem % 1000, 3)}Z`;
}
