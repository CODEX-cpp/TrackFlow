import moment, { Moment, Duration } from "moment";
import { useSettingsStore } from "~/stores/settings";

function getStartOfDayOffset() {
  const settingsStore = useSettingsStore();
  return settingsStore.startOfDay;
}

function normalize_date(dateParam: Date) {
  return new Date(dateParam.getTime());
}

export function seconds_to_duration(seconds: number) {
  // Returns a human-readable duration string
  const hrs = Math.floor(seconds / 60 / 60);
  const min = Math.floor((seconds / 60) % 60);
  const sec = Math.floor(seconds % 60);
  const l = [];

  if (hrs > 0) {
    l.push(hrs + "h");
    l.push(min + "m");
  } else if (min > 0) {
    l.push(min + "m");
  }
  l.push(sec + "s");

  return l.join(" ");
}

export function friendlydate(timestamp) {
  const now = moment();
  const m = moment.parseZone(timestamp);
  const sinceNow = moment.duration(m.diff(now));
  const secondsAgo = -sinceNow.asSeconds();

  if (Math.abs(secondsAgo) <= 60) {
    // Recente, in entrambe le direzioni (passato o futuro)
    return secondsAgo >= 0
      ? `${Math.round(secondsAgo)}s ago`
      : `in ${Math.round(-secondsAgo)}s`;
  }
  // Oltre il minuto, lasciamo fare a moment.js (gestisce già bene sia
  // passato "2 hours ago" che futuro "in 2 hours")
  return sinceNow.humanize(true);
}

export function get_day_start_with_offset(
  dateParam: Moment | string,
  offset?: string,
) {
  if (!offset) {
    offset = getStartOfDayOffset();
  }
  const dateMoment = dateParam ? moment(dateParam) : moment().startOf("day");
  const start_of_day_hours = parseInt(offset.split(":")[0]);
  const start_of_day_minutes = parseInt(offset.split(":")[1]);
  return dateMoment
    .hour(start_of_day_hours)
    .minute(start_of_day_minutes)
    .format();
}

// Return the startOfDay offset as a number of hours
export function get_hour_offset(offset?: string): number {
  if (!offset) {
    offset = getStartOfDayOffset();
  }
  const start_of_day_hours = parseInt(offset.split(":")[0]);
  const start_of_day_minutes = parseInt(offset.split(":")[1]);
  return start_of_day_hours + start_of_day_minutes / 60;
}

export function get_day_end_with_offset(
  date: Moment | string,
  offset?: string,
): string {
  if (!offset) {
    offset = getStartOfDayOffset();
  }
  return moment(get_day_start_with_offset(date, offset))
    .add(1, "days")
    .format();
}

export function get_offset_duration(offset?: string): Duration {
  if (!offset) {
    offset = getStartOfDayOffset();
  }
  const [hours, minutes] = offset.split(":");
  return moment.duration({ hours: Number(hours), minutes: Number(minutes) });
}

export function get_today_with_offset(offset?: string): string {
  if (!offset) {
    offset = getStartOfDayOffset();
  }
  // Gets "today" in an offset-aware way
  const offset_dur = get_offset_duration(offset);
  return moment().subtract(offset_dur).startOf("day").format("YYYY-MM-DD");
}

export function format_weekday_short(
  dateParam: Date,
  locale?: string | string[],
) {
  return new Intl.DateTimeFormat(locale, { weekday: "short" }).format(
    normalize_date(dateParam),
  );
}

export function format_day_of_month(
  dateParam: Date,
  locale?: string | string[],
) {
  return new Intl.DateTimeFormat(locale, { day: "numeric" }).format(
    normalize_date(dateParam),
  );
}

export function get_short_month_labels(locale?: string | string[]) {
  return Array.from({ length: 12 }, (_, month) =>
    new Intl.DateTimeFormat(locale, { month: "short" }).format(
      new Date(2020, month, 1, 12),
    ),
  );
}

// Format a time-of-day string locale-aware (e.g. "14:30:05" or "2:30:05 PM" per browser locale)
export function format_time_of_day(
  date: Date | string,
  locale?: string | string[],
) {
  const d = date instanceof Date ? date : new Date(date);
  return new Intl.DateTimeFormat(locale, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(d);
}
