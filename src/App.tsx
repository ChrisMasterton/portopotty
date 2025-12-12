import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { loadRanges, saveRanges, type PortRange } from "./storage";

type Listener = {
  port: number;
  pid: number;
  process_name?: string | null;
  started_seconds_ago?: number | null;
};

type SortKey = "port" | "process" | "pid" | "started";
type SortDir = "asc" | "desc";

function formatUptime(secondsAgo?: number | null) {
  if (secondsAgo == null) return "—";
  if (secondsAgo < 60) return `${secondsAgo}s ago`;
  const minutes = Math.floor(secondsAgo / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function compareNullable<T>(a: T | null | undefined, b: T | null | undefined, compare: (x: T, y: T) => number) {
  if (a == null && b == null) return 0;
  if (a == null) return 1;
  if (b == null) return -1;
  return compare(a, b);
}

function normalizeRanges(ranges: PortRange[]) {
  return ranges
    .map((r) => ({
      start: Math.max(1, Math.min(65535, Math.floor(r.start))),
      end: Math.max(1, Math.min(65535, Math.floor(r.end)))
    }))
    .map((r) => (r.start <= r.end ? r : { start: r.end, end: r.start }));
}

export default function App() {
  const [ranges, setRanges] = useState<PortRange[]>(() => normalizeRanges(loadRanges()));
  const [start, setStart] = useState<number>(3000);
  const [end, setEnd] = useState<number>(3999);
  const [listeners, setListeners] = useState<Listener[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [inFocus, setInFocus] = useState(true);
  const [sortKey, setSortKey] = useState<SortKey>("port");
  const [sortDir, setSortDir] = useState<SortDir>("asc");

  const rangesRef = useRef(ranges);
  const didMount = useRef(false);
  useEffect(() => {
    rangesRef.current = ranges;
    saveRanges(ranges);

    if (didMount.current) {
      refresh();
    } else {
      didMount.current = true;
    }
  }, [ranges]);

  const rangesLabel = useMemo(
    () => ranges.map((r) => `${r.start}-${r.end}`).join(", "),
    [ranges]
  );

  const refresh = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const data = await invoke<Listener[]>("scan_ports", { ranges: rangesRef.current });
      const dedupKey = (l: Listener) => `${l.port}:${l.pid}`;
      const uniq = new Map<string, Listener>();
      for (const l of data) uniq.set(dedupKey(l), l);
      setListeners(Array.from(uniq.values()));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    function updateFocus() {
      const visible = document.visibilityState === "visible";
      setInFocus(visible);
    }
    updateFocus();
    document.addEventListener("visibilitychange", updateFocus);
    window.addEventListener("focus", updateFocus);
    window.addEventListener("blur", updateFocus);
    return () => {
      document.removeEventListener("visibilitychange", updateFocus);
      window.removeEventListener("focus", updateFocus);
      window.removeEventListener("blur", updateFocus);
    };
  }, []);

  useEffect(() => {
    if (!inFocus) return;
    const id = window.setInterval(() => {
      refresh();
    }, 15_000);
    return () => window.clearInterval(id);
  }, [inFocus, refresh]);

  async function kill(pid: number) {
    const ok = confirm(`Kill PID ${pid}?`);
    if (!ok) return;
    setError(null);
    try {
      await invoke("kill_pid", { pid });
      setListeners((prev) => prev.filter((l) => l.pid !== pid));
      // also refresh soon to catch port rebinds
      setTimeout(() => refresh(), 600);
    } catch (e) {
      setError(String(e));
    }
  }

  function addRange() {
    const next = normalizeRanges([...ranges, { start, end }]);
    setRanges(next);
  }

  function removeRange(index: number) {
    setRanges((prev) => prev.filter((_, i) => i !== index));
  }

  const sortedListeners = useMemo(() => {
    const dir = sortDir === "asc" ? 1 : -1;
    const byNumber = (a: number, b: number) => (a - b) * dir;
    const byString = (a: string, b: string) => a.localeCompare(b, undefined, { sensitivity: "base" }) * dir;
    const byUptime = (a: number, b: number) => (a - b) * dir;

    const base = listeners.map((l, idx) => ({ l, idx }));
    base.sort((aa, bb) => {
      const a = aa.l;
      const b = bb.l;

      let cmp = 0;
      switch (sortKey) {
        case "port":
          cmp = byNumber(a.port, b.port);
          break;
        case "pid":
          cmp = byNumber(a.pid, b.pid);
          break;
        case "process":
          cmp = compareNullable(a.process_name ?? null, b.process_name ?? null, byString);
          break;
        case "started":
          cmp = compareNullable(a.started_seconds_ago ?? null, b.started_seconds_ago ?? null, byUptime);
          break;
      }

      if (cmp !== 0) return cmp;
      // deterministic tie-breakers
      cmp = a.port - b.port;
      if (cmp !== 0) return cmp;
      cmp = a.pid - b.pid;
      if (cmp !== 0) return cmp;
      return aa.idx - bb.idx;
    });
    return base.map((x) => x.l);
  }, [listeners, sortDir, sortKey]);

  function toggleSort(nextKey: SortKey) {
    if (nextKey === sortKey) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortKey(nextKey);
      setSortDir("asc");
    }
  }

  function sortIndicator(key: SortKey) {
    if (key !== sortKey) return "↕";
    return sortDir === "asc" ? "↑" : "↓";
  }

  return (
    <div className="container">
      <div className="header">
        <div>
          <div className="title">Port-o-Potty</div>
          <div className="subtitle">Shows listeners in your configured port ranges.</div>
        </div>
        <div className="pill">
          <span>{inFocus ? "Active" : "Paused"}</span>
          <span className="muted">•</span>
          <span className="muted">Refresh: 15s</span>
        </div>
      </div>

      {error ? <div className="error">{error}</div> : null}

      <div className="grid">
        <div className="panel">
          <h2>Port Ranges</h2>
          <div className="row">
            <input
              type="number"
              min={1}
              max={65535}
              value={start}
              onChange={(e) => setStart(Number(e.target.value))}
            />
            <span className="muted">to</span>
            <input
              type="number"
              min={1}
              max={65535}
              value={end}
              onChange={(e) => setEnd(Number(e.target.value))}
            />
          </div>
          <div className="row">
            <button className="btn primary" onClick={addRange}>
              Add range
            </button>
            <button className="btn" onClick={refresh} disabled={busy}>
              {busy ? "Refreshing…" : "Refresh now"}
            </button>
          </div>

          <div className="muted" style={{ marginBottom: 10, fontSize: 12 }}>
            Watching: {rangesLabel || "—"}
          </div>

          {ranges.length === 0 ? (
            <div className="empty">Add at least one port range.</div>
          ) : (
            <table className="table">
              <thead>
                <tr>
                  <th>Range</th>
                  <th style={{ width: 120 }} />
                </tr>
              </thead>
              <tbody>
                {ranges.map((r, i) => (
                  <tr key={`${r.start}-${r.end}-${i}`}>
                    <td>
                      {r.start}–{r.end}
                    </td>
                    <td>
                      <button className="btn danger" onClick={() => removeRange(i)}>
                        Remove
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        <div className="panel">
          <h2>Listeners</h2>
          {sortedListeners.length === 0 ? (
            <div className="empty">
              No listeners found in these ranges. {busy ? "Scanning…" : ""}
            </div>
          ) : (
            <table className="table">
              <thead>
                <tr>
                  <th style={{ width: 90 }}>
                    <button
                      className="thbtn"
                      onClick={() => toggleSort("port")}
                      type="button"
                      aria-sort={sortKey === "port" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                    >
                      Port <span className="thicon">{sortIndicator("port")}</span>
                    </button>
                  </th>
                  <th>
                    <button
                      className="thbtn"
                      onClick={() => toggleSort("process")}
                      type="button"
                      aria-sort={sortKey === "process" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                    >
                      Process <span className="thicon">{sortIndicator("process")}</span>
                    </button>
                  </th>
                  <th style={{ width: 90 }}>
                    <button
                      className="thbtn"
                      onClick={() => toggleSort("pid")}
                      type="button"
                      aria-sort={sortKey === "pid" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                    >
                      PID <span className="thicon">{sortIndicator("pid")}</span>
                    </button>
                  </th>
                  <th style={{ width: 120 }}>
                    <button
                      className="thbtn"
                      onClick={() => toggleSort("started")}
                      type="button"
                      aria-sort={sortKey === "started" ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
                    >
                      Started <span className="thicon">{sortIndicator("started")}</span>
                    </button>
                  </th>
                  <th style={{ width: 110 }} />
                </tr>
              </thead>
              <tbody>
                {sortedListeners.map((l) => (
                  <tr key={`${l.port}:${l.pid}`}>
                    <td>{l.port}</td>
                    <td>{l.process_name ?? "—"}</td>
                    <td className="muted">{l.pid}</td>
                    <td className="muted">{formatUptime(l.started_seconds_ago)}</td>
                    <td>
                      <button className="btn danger" onClick={() => kill(l.pid)}>
                        Kill
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>
    </div>
  );
}
