// Extracts a bare domain ("github.com", not "www.github.com") from a
// browser-watcher event's URL, falling back to the tab title if the
// URL is missing or unparseable. Shared by the Home timeline's Browser
// lane and the "Top Domini Browser" summary panel.
export function domainForEvent(event: any): string {
  const url = event.data && event.data.url;
  if (!url) return event.data?.title || 'Sconosciuto';
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return event.data?.title || 'Sconosciuto';
  }
}
