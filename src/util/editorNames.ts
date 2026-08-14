// Shared parsing for aw-watcher-vscode's raw `project`/`file` fields —
// full filesystem paths, using '/' on Linux/Mac and '\' on Windows.
// Used by both the Top Editor Files/Projects summary panels
// (SelectableVisualization.vue) and the Timeline's VSCode lane
// (HomeTimelineSection.vue), so a row click's key always matches the
// lane's block key exactly.
function basename(path: string): string {
  const parts = (path || '').split(/[/\\]/);
  return parts[parts.length - 1] || '';
}

export function projectDisplayName(projectPath: string): string {
  return basename(projectPath);
}

export function fileDisplayName(filePath: string): string {
  return basename(filePath);
}

// The extension sends a heartbeat with every field set to the literal
// string "unknown" whenever no file has focus (e.g. focus moved to a
// terminal panel or another app entirely) — not real editor activity,
// so callers filter these out instead of showing/drawing a "Sconosciuto"
// row or block for them.
export function isKnownEditorValue(value: string): boolean {
  return !!value && value !== 'unknown';
}
