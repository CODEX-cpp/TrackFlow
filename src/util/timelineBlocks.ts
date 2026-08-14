// Pure block-building/layout math for the Home Timeline — split out of
// HomeTimelineSection.vue once that file crossed 1000 lines (same
// reasoning as the Progetti.vue decomposition, see BLUEPRINT.md section
// 7.3): none of this depends on Vue reactivity, it's just moment.js
// math taking plain values in and returning plain values out, so it's
// easy to reuse (e.g. from titleHighlightRanges' own re-merge) and to
// reason about independently of the component's template/DOM concerns.
import moment from 'moment';
import { colorVarForName } from '~/util/hashColor';

export interface TimeRange {
  start: moment.Moment;
  end: moment.Moment;
}

export interface KeyedRange extends TimeRange {
  key: string;
}

export interface RowAssignedRange extends KeyedRange {
  row: number;
}

export interface Block extends RowAssignedRange {
  left: number;
  width: number;
  color: string;
}

// Merges same-key events into one continuous range whenever the gap
// between one ending and the next starting is under `mergeGapSeconds`
// — grouped by key FIRST (not just "adjacent in time" across every key
// at once), so a brief glance at a different app in between two Zen
// Browser moments doesn't reset the run. Without this, alt-tabbing away
// for even a couple of seconds and back split what's clearly "the same
// stretch of browsing" into a wall of tiny separate occurrences (real
// bug reported: dozens of few-second Zen Browser entries a minute or
// two apart, none merging, because an earlier version only ever
// compared each event to the single immediately-preceding one
// regardless of key).
export function mergeEventsByKey(
  events: any[],
  keyFn: (e: any) => string,
  mergeGapSeconds: number
): KeyedRange[] {
  const byKey = new Map<string, TimeRange[]>();
  for (const e of events) {
    const start = moment(e.timestamp);
    const end = start.clone().add(e.duration || 0, 'seconds');
    const key = keyFn(e);
    const existing = byKey.get(key);
    if (existing) {
      existing.push({ start, end });
    } else {
      byKey.set(key, [{ start, end }]);
    }
  }

  const merged: KeyedRange[] = [];
  for (const [key, occurrences] of byKey) {
    occurrences.sort((a, b) => a.start.diff(b.start));
    let current: KeyedRange | null = null;
    for (const occ of occurrences) {
      if (current && occ.start.diff(current.end, 'seconds') <= mergeGapSeconds) {
        if (occ.end.isAfter(current.end)) current.end = occ.end;
      } else {
        if (current) merged.push(current);
        current = { key, start: occ.start, end: occ.end };
      }
    }
    if (current) merged.push(current);
  }
  return merged;
}

function overlapsAnother(range: TimeRange, all: TimeRange[]): boolean {
  return all.some(o => o !== range && o.start.isBefore(range.end) && range.start.isBefore(o.end));
}

// Cleanup pass, explicit request: a short range is only dropped when it
// time-overlaps another range in the same lane — a real (if brief)
// standalone activity like a 22s Task Manager check stays visible;
// what gets cut is specifically the case where a short blip is visual
// clutter sitting on top of/beside a longer block. (Briefly made
// unconditional — dropping every short range regardless of overlap —
// which also hid genuine brief activity from both the Timeline and the
// block-detail popup's "Altre occorrenze" list; reverted on request.
// Known tradeoff: a short, non-overlapping, near-zero-duration blip
// (e.g. a single stray heartbeat) can still show up as a "00:00:00" row
// there, same as before this whole cleanup pass existed.)
export function dropShortOverlappingRanges<T extends TimeRange>(
  ranges: T[],
  minOverlappingSeconds: number
): T[] {
  return ranges.filter(
    r => r.end.diff(r.start, 'seconds') >= minOverlappingSeconds || !overlapsAnother(r, ranges)
  );
}

// Packs ranges that overlap in time onto up to `maxRows` sub-rows —
// greedy, sorted by start time, so the earlier-starting range always
// claims the first free row (explicit request: the one that starts
// first stays on top). If more than `maxRows` ranges genuinely overlap
// at once (rare), the extras double up on the last row rather than
// growing further — accepted as a rare-case tradeoff, not a case
// anyone asked to handle "correctly".
export function assignRows<T extends KeyedRange>(
  ranges: T[],
  maxRows: number
): (T & { row: number })[] {
  const sorted = [...ranges].sort((a, b) => a.start.diff(b.start));
  const trackEnds: (moment.Moment | null)[] = Array(maxRows).fill(null);
  return sorted.map(r => {
    let row = maxRows - 1;
    for (let i = 0; i < maxRows; i++) {
      const trackEnd = trackEnds[i];
      if (!trackEnd || !r.start.isBefore(trackEnd)) {
        row = i;
        break;
      }
    }
    const currentEnd = trackEnds[row];
    trackEnds[row] = currentEnd && currentEnd.isAfter(r.end) ? currentEnd : r.end;
    return { ...r, row };
  });
}

// Pixel left/width only — split out so a zoom-level change can re-lay
// out already-fetched blocks without also re-deriving `color`, which
// depends on which colorFn built the block in the first place and must
// survive re-layouts untouched.
export function blockExtent(
  range: TimeRange,
  viewStart: moment.Moment,
  viewEnd: moment.Moment,
  pxPerMinute: number
): { left: number; width: number } {
  const clampedStart = moment.max(range.start, viewStart);
  const clampedEnd = moment.min(range.end, viewEnd);
  const left = clampedStart.diff(viewStart, 'minutes') * pxPerMinute;
  const width = Math.max(3, clampedEnd.diff(clampedStart, 'minutes') * pxPerMinute);
  return { left, width };
}

export function layoutBlock(
  range: RowAssignedRange,
  viewStart: moment.Moment,
  viewEnd: moment.Moment,
  pxPerMinute: number,
  colorFn: (key: string) => string = colorVarForName
): Block {
  const { left, width } = blockExtent(range, viewStart, viewEnd, pxPerMinute);
  return { ...range, left, width, color: colorFn(range.key) };
}

export function rowTop(
  row: number,
  blockHeight: number,
  rowGap: number,
  trackPadding: number
): number {
  return trackPadding + row * (blockHeight + rowGap);
}

export function trackHeight(
  maxRow: number,
  blockHeight: number,
  rowGap: number,
  trackPadding: number
): number {
  return trackPadding * 2 + (maxRow + 1) * blockHeight + maxRow * rowGap;
}

// Clips raw events down to only the portions that fall inside one of
// `allowed` (typically the day's not-afk intervals from the AFK
// watcher) — an event straddling an AFK gap is split into the pieces on
// either side, one entirely inside an AFK gap is dropped, one only
// partly inside is shortened. Applied to every lane's raw events before
// merging (see HomeTimelineSection.vue's load()), so a long stretch of
// e.g. Claude Code heartbeats that happens to span a real AFK period
// (explicit bug report: leaving the PC running from 12:30 to 15:30
// still showed one unbroken "Claude" bar all afternoon) shows the real
// gap instead of reading as continuous work.
// Zero-duration events (a single heartbeat with nothing to merge it
// into yet) are treated as an instant: kept only if that instant falls
// inside an allowed interval, boundaries inclusive.
// Inverso di clipEventsToIntervals(): rimuove dagli eventi le porzioni
// che ricadono dentro uno qualunque di `covering`, tenendo solo quello
// che resta fuori — un evento parzialmente coperto viene accorciato
// (o spezzato in due, se `covering` cade nel mezzo). Usato per il bug
// segnalato dall'utente nella corsia Claude: la barra generica "Claude
// (finestra)" (semplice focus della finestra, nessun progetto/sessione
// noto) veniva mostrata SEMPRE insieme alla barra della sessione vera
// (nome cliente/progetto reale), quasi sempre nello stesso orario —
// due barre quasi identiche, non chiaro a cosa servisse la seconda.
// Sottraendo qui gli intervalli già coperti da una sessione vera, la
// barra "finestra" resta visibile solo per i tratti in cui Claude
// Desktop era aperta ma nessuna sessione specifica era in corso.
export function subtractIntervals(events: any[], covering: TimeRange[]): any[] {
  if (covering.length === 0) return events;
  const sorted = [...covering].sort((a, b) => a.start.diff(b.start));
  const result: any[] = [];
  for (const e of events) {
    const start = moment(e.timestamp);
    const end = start.clone().add(e.duration || 0, 'seconds');
    let segments: TimeRange[] = [{ start, end }];
    for (const c of sorted) {
      const next: TimeRange[] = [];
      for (const seg of segments) {
        if (!c.start.isBefore(seg.end) || !c.end.isAfter(seg.start)) {
          // Nessuna sovrapposizione: il segmento resta intatto.
          next.push(seg);
          continue;
        }
        if (c.start.isAfter(seg.start)) {
          next.push({ start: seg.start, end: c.start });
        }
        if (c.end.isBefore(seg.end)) {
          next.push({ start: c.end, end: seg.end });
        }
      }
      segments = next;
    }
    for (const seg of segments) {
      if (seg.end.isAfter(seg.start) || (seg.end.isSame(seg.start) && end.isSame(start))) {
        result.push({
          ...e,
          timestamp: seg.start.toISOString(),
          duration: seg.end.diff(seg.start, 'seconds'),
        });
      }
    }
  }
  return result;
}

export function clipEventsToIntervals(events: any[], allowed: TimeRange[]): any[] {
  if (allowed.length === 0) return events;
  const result: any[] = [];
  for (const e of events) {
    const start = moment(e.timestamp);
    const end = start.clone().add(e.duration || 0, 'seconds');
    if (end.isSame(start)) {
      if (allowed.some(a => !start.isBefore(a.start) && !start.isAfter(a.end))) {
        result.push(e);
      }
      continue;
    }
    for (const a of allowed) {
      if (a.start.isSameOrAfter(end) || a.end.isSameOrBefore(start)) continue;
      const clippedStart = moment.max(start, a.start);
      const clippedEnd = moment.min(end, a.end);
      if (clippedEnd.isAfter(clippedStart)) {
        result.push({
          ...e,
          timestamp: clippedStart.toISOString(),
          duration: clippedEnd.diff(clippedStart, 'seconds'),
        });
      }
    }
  }
  return result;
}

// Cheap "did anything actually change" fingerprint for a set of event
// lists — count + id/duration of both the first and last item per list,
// joined. Checks both ends deliberately: the ActivityWatch API returns
// events newest-first, so the "currently in progress" event (the one
// that matters most for staying live — its duration keeps growing in
// place, same id, as long as heartbeats keep landing within pulsetime)
// sits at index 0, not at the end. An earlier version only fingerprinted
// the last (oldest) item's id, which barely ever changes — so a poll
// while an event was still actively extending looked identical to the
// last one and got silently skipped, freezing the Timeline (and
// anything derived from "how recent is the latest activity", like the
// AFK status bar's end) at a stale snapshot instead of tracking "now".
// Checking both ends keeps this correct even if the API's sort order
// ever changes. Not a true deep diff (a same-count edit to a
// *non-boundary* old event would slip through unnoticed), but
// ActivityWatch events are effectively append-only/extend-in-place in
// normal use, so this catches every case that actually happens in
// practice at a fraction of the cost of re-running the full merge/
// layout pipeline on identical data on every auto-refresh poll.
export function eventListSignature(lists: any[][]): string {
  return lists
    .map(list => {
      const first = list[0];
      const last = list[list.length - 1];
      return `${list.length}:${first?.id ?? ''}:${first?.duration ?? ''}:${last?.id ?? ''}:${last?.duration ?? ''}`;
    })
    .join('|');
}
