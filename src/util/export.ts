/**
 * Cross-platform file download utility.
 *
 * In a regular browser, uses the standard <a download> pattern.
 * In a Tauri webview, uses the dialog + fs plugins to show a native
 * save dialog and write the file directly (webviews don't support
 * the <a download> pattern).
 *
 * See: https://github.com/ActivityWatch/aw-tauri/issues/199
 */

import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';

interface FileFilter {
  name: string;
  extensions: string[];
}

function isTauri(): boolean {
  // '__TAURI__' esiste solo se `app.withGlobalTauri` è attivo in
  // tauri.conf.json (qui non lo è) — '__TAURI_INTERNALS__' è invece il
  // bridge IPC interno che @tauri-apps/api/i plugin usano sempre,
  // indipendentemente da quell'opzione, quindi è il modo giusto per
  // rilevare "sto girando dentro Tauri" senza doverla attivare solo per
  // questo. Bug reale: con '__TAURI__' il controllo era sempre falso, e
  // downloadFile() finiva SEMPRE nel metodo browser (che non funziona in
  // una webview Tauri, vedi sopra) anche con i permessi dialog/fs a posto.
  return '__TAURI_INTERNALS__' in window;
}

/**
 * Download/save a file with the given content.
 *
 * @param filename - Default filename for the download
 * @param content - File content as a string
 * @param mimeType - MIME type (e.g. 'application/json', 'text/csv')
 */
export async function downloadFile(
  filename: string,
  content: string,
  mimeType: string
): Promise<void> {
  if (isTauri()) {
    await downloadFileTauri(filename, content, mimeType);
  } else {
    downloadFileBrowser(filename, content, mimeType);
  }
}

function getFilters(mimeType: string): FileFilter[] {
  if (mimeType.includes('json')) {
    return [{ name: 'JSON', extensions: ['json'] }];
  } else if (mimeType.includes('csv')) {
    return [{ name: 'CSV', extensions: ['csv'] }];
  }
  return [];
}

async function downloadFileTauri(
  filename: string,
  content: string,
  mimeType: string
): Promise<void> {
  try {
    const path = await save({
      title: 'Save export',
      defaultPath: filename,
      filters: getFilters(mimeType),
    });

    if (path) {
      await writeTextFile(path, content);
    }
  } catch (e) {
    console.warn('Tauri save failed, falling back to browser download:', e);
    // Fall back to browser method if Tauri plugins aren't available
    downloadFileBrowser(filename, content, mimeType);
  }
}

function downloadFileBrowser(filename: string, content: string, mimeType: string): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  link.style.display = 'none';
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}
