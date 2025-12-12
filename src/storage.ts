export type PortRange = { start: number; end: number };

const KEY = "port_o_potty_ranges_v1";

export function loadRanges(): PortRange[] {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return [{ start: 3000, end: 3999 }, { start: 8000, end: 8999 }];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) throw new Error("invalid");
    return parsed
      .map((r) => ({
        start: Number((r as any).start),
        end: Number((r as any).end)
      }))
      .filter((r) => Number.isFinite(r.start) && Number.isFinite(r.end));
  } catch {
    return [{ start: 3000, end: 3999 }, { start: 8000, end: 8999 }];
  }
}

export function saveRanges(ranges: PortRange[]) {
  localStorage.setItem(KEY, JSON.stringify(ranges));
}

