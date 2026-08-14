// Pure time-formatting helpers shared by the Projects page components.
// No Vue state, no dependency on any component — safe to import from
// anywhere (views, components, tests).

import moment from 'moment';

// "02:15:09" — full stopwatch-style readout used on cards and totals.
export function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(hours)}:${pad(minutes)}:${pad(secs)}`;
}

// "2h 15m" — short label used for overage amounts, where a full
// HH:MM:SS readout would be noisy.
export function formatHoursMinutes(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.round((seconds % 3600) / 60);
  if (hours === 0) return `${minutes}m`;
  if (minutes === 0) return `${hours}h`;
  return `${hours}h ${minutes}m`;
}

// "7 agosto 2026" — long Italian date label used for deadlines and
// closed-on dates throughout the Projects page.
export function formatLongDate(iso: string): string {
  return moment(iso).locale('it').format('D MMMM YYYY');
}
