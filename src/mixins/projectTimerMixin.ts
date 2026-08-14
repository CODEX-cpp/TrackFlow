// Shared live project-timer logic — reads/writes the `aw-stopwatch`
// bucket, keeps a ticking clock for live durations, and exposes
// play/pause/close/overage-check behaviour. Used by both the Projects
// page and the "Active projects" card on Home, so it lives here once
// instead of being duplicated (or, worse, drifting apart) between them.
//
// A classic Vue 2 mixin rather than a Pinia store on purpose: this data
// is live/derived from ActivityWatch events, not persisted config — the
// project metadata store (`useProjectsStore`) deliberately stays
// metadata-only, see BLUEPRINT.md section 7.1.
import Vue from 'vue';
import moment from 'moment';
import { invoke } from '@tauri-apps/api/core';
import { useProjectsStore, Project } from '~/stores/projects';
import { formatDuration, formatHoursMinutes, formatLongDate } from '~/util/projectTime';
import { getHomeClient } from '~/util/awclient';

const BUCKET_ID = 'aw-stopwatch';

// A "segment" is one continuous stretch of work: start, and end (null
// if it's the one currently running).
export interface Segment {
  start: string; // ISO
  end: string | null; // ISO, or null if still open
}

// A "block" is an ActivityWatch event representing one work period on a
// project, which can contain multiple segments (with pauses in between).
// "sealed" = true once the project has been closed while this was the
// current block: a future Play, after reopening the project, must create
// a brand new block instead of resuming this one.
export interface BlockData {
  projectId: string;
  segments: Segment[];
  sealed: boolean;
}

export default Vue.extend({
  data() {
    return {
      projectsStore: useProjectsStore(),
      // All events (blocks) read from the aw-stopwatch bucket, for
      // every project at once — filtered/grouped locally instead of
      // making one request per project.
      allBlocks: [] as any[],
      loading: true,
      // Ticking clock, to keep the live duration on in-progress cards
      // updating.
      now: moment(),
      tickInterval: null as ReturnType<typeof setInterval> | null,
      // Project waiting on an "start anyway" confirmation because it's
      // over budget/deadline — null when no popup is open.
      pendingPlayProject: null as Project | null,
      // Chiave (id progetto + testo esatto dello sforamento) per cui è
      // già partita una notifica di sistema in questa sessione — solo in
      // memoria, si resetta ad ogni riavvio dell'app (stesso
      // comportamento della notifica VPN, vedi vpn_notify.rs). Il testo
      // fa parte della chiave apposta: se lo sforamento cambia (es. da
      // "solo ore" a "ore + scadenza"), è un'informazione nuova e vale
      // la pena notificare di nuovo.
      progettiGiaNotificati: new Set<string>(),
    };
  },
  computed: {
    // Projects to show as a big card up top: started and not yet
    // closed — stay visible whether running or paused, until you press
    // "Chiudi progetto" (which seals the block and drops them from here).
    playingProjects(): Project[] {
      return this.projectsStore.activeProjects.filter((p: Project) => this.currentBlockFor(p) !== null);
    },
    pendingPlayLines(): string[] {
      return this.pendingPlayProject ? this.overageLines(this.pendingPlayProject) : [];
    },
  },
  async mounted() {
    await this.projectsStore.load();
    await getHomeClient().ensureBucket(BUCKET_ID, 'general.stopwatch', 'unknown');
    await this.fetchBlocks();
    this.loading = false;

    this.tickInterval = setInterval(() => {
      this.now = moment();
    }, 1000);
  },
  beforeDestroy() {
    if (this.tickInterval) clearInterval(this.tickInterval);
  },
  methods: {
    formatDuration,
    formatLongDate,

    async fetchBlocks() {
      this.allBlocks = await getHomeClient().getEvents(BUCKET_ID, { limit: -1 });
      this.checkOverageNotifications();
    },

    // Manda una notifica di sistema (toast Windows, vedi
    // notifications.rs) la prima volta che un progetto attivo risulta
    // fuori budget ore o oltre la scadenza — non ad ogni singolo
    // fetchBlocks() (ogni 30s, vedi ActiveProjectsSection.vue), solo la
    // prima volta che lo sforamento (in questa forma esatta) compare.
    // Chiamata da fetchBlocks() così copre sia il primo caricamento sia
    // ogni refresh periodico successivo con un solo punto d'ingresso.
    checkOverageNotifications() {
      for (const project of this.projectsStore.activeProjects) {
        const lines = this.overageLines(project);
        if (lines.length === 0) continue;
        const chiave = `${project.id}|${lines.join('|')}`;
        if (this.progettiGiaNotificati.has(chiave)) continue;
        this.progettiGiaNotificati.add(chiave);
        invoke('invia_notifica_generica', {
          titolo: this.$t('projects.overageNotificationTitle', { name: project.name }) as string,
          corpo: lines.join(' '),
        }).catch(() => undefined);
      }
    },

    // The "current" block for a project: the most recent NON-sealed
    // one. If none exists (never started, or the last one was sealed
    // by closing the project), the next Play creates a fresh one.
    currentBlockFor(project: Project) {
      const blocks = this.allBlocks
        .filter((e: any) => e.data.projectId === project.id && !e.data.sealed)
        .sort((a: any, b: any) => moment(b.timestamp).diff(moment(a.timestamp)));
      return blocks[0] || null;
    },

    isPlaying(project: Project): boolean {
      const block = this.currentBlockFor(project);
      if (!block) return false;
      const segments: Segment[] = block.data.segments;
      const last = segments[segments.length - 1];
      return last && last.end === null;
    },

    // Total duration of a block, summing all its segments (the one
    // still open counts up to "now").
    durataBlocco(block: any): number {
      const segments: Segment[] = block.data.segments;
      let total = 0;
      for (const s of segments) {
        const end = s.end ? moment(s.end) : this.now;
        total += end.diff(moment(s.start), 'seconds');
      }
      return total;
    },

    liveSecondsFor(project: Project): number {
      const block = this.currentBlockFor(project);
      return block ? this.durataBlocco(block) : 0;
    },

    // Aggregate stats for a card: all-time total, today's total, last
    // session label.
    statsFor(project: Project) {
      const projectBlocks = this.allBlocks.filter((e: any) => e.data.projectId === project.id);

      let totalSeconds = 0;
      let todaySeconds = 0;
      let lastSegmentEnd: moment.Moment | null = null;
      const today = moment().format('YYYY-MM-DD');

      for (const block of projectBlocks) {
        for (const s of block.data.segments as Segment[]) {
          const start = moment(s.start);
          const end = s.end ? moment(s.end) : this.now;
          const duration = end.diff(start, 'seconds');
          totalSeconds += duration;
          if (start.format('YYYY-MM-DD') === today) {
            todaySeconds += duration;
          }
          if (!lastSegmentEnd || end.isAfter(lastSegmentEnd)) {
            lastSegmentEnd = end;
          }
        }
      }

      let lastSessionLabel = this.$t('projects.lastSessionNever') as string;
      if (this.isPlaying(project)) {
        lastSessionLabel = this.$t('projects.lastSessionInProgress') as string;
      } else if (lastSegmentEnd) {
        lastSessionLabel = this.$t('projects.lastSessionPrefix') + ' ' + lastSegmentEnd.fromNow();
      }

      return { totalSeconds, todaySeconds, lastSessionLabel };
    },

    async togglePlay(project: Project) {
      const block = this.currentBlockFor(project);
      const aboutToStart = !block || block.data.segments[block.data.segments.length - 1].end !== null;

      // Only ask for confirmation when the action is about to START
      // the timer (fresh start, or resume from pause) on an already
      // over-budget/overdue project — never when the action is pausing.
      if (aboutToStart && this.overageLines(project).length > 0) {
        this.pendingPlayProject = project;
        return;
      }

      await this.performToggle(project);
    },
    // The actual block mutation — split out from togglePlay so it can
    // be called either directly (no overage) or from the confirm popup
    // after a "start anyway".
    async performToggle(project: Project) {
      const block = this.currentBlockFor(project);

      if (!block) {
        // No open block for this project: create a new one with a
        // first segment.
        const newEvent = {
          timestamp: new Date(),
          data: {
            projectId: project.id,
            sealed: false,
            segments: [{ start: new Date().toISOString(), end: null }],
          } as BlockData,
        };
        // Cast: BlockData's `segments` array doesn't fit AWClient's
        // strict EventData typing (same reason the old `this.$aw` call
        // this replaced was implicitly `any` via `(this as any).$aw`).
        await (getHomeClient() as any).heartbeat(BUCKET_ID, 1, newEvent);
      } else {
        const segments: Segment[] = block.data.segments;
        const last = segments[segments.length - 1];

        if (last && last.end === null) {
          // Running: Pause — close the open segment.
          const updated = JSON.parse(JSON.stringify(block));
          updated.data.segments[updated.data.segments.length - 1].end = new Date().toISOString();
          await getHomeClient().replaceEvent(BUCKET_ID, updated);
        } else {
          // Paused: Resume — open a new segment in the same block.
          const updated = JSON.parse(JSON.stringify(block));
          updated.data.segments.push({ start: new Date().toISOString(), end: null });
          await getHomeClient().replaceEvent(BUCKET_ID, updated);
        }
      }

      await this.fetchBlocks();
    },

    async closeProject(project: Project) {
      const block = this.currentBlockFor(project);
      if (block) {
        const updated = JSON.parse(JSON.stringify(block));
        const segments: Segment[] = updated.data.segments;
        const last = segments[segments.length - 1];
        // If it was still running, pause it before sealing.
        if (last && last.end === null) {
          last.end = new Date().toISOString();
        }
        updated.data.sealed = true;
        await getHomeClient().replaceEvent(BUCKET_ID, updated);
        await this.fetchBlocks();
      }

      this.projectsStore.closeProject({ id: project.id });
      await this.projectsStore.save();
    },

    async onAddProject() {
      const n = this.projectsStore.projects.length + 1;
      const name = (this.$t('projects.newProjectDefaultName', { n }) as string) || `New project ${n}`;
      this.projectsStore.addProject({ name });
      await this.projectsStore.save();
    },

    // "Over" = worked time beyond the hour budget, or today is past the
    // expected completion date (and the project is still open — no
    // point warning on an already-closed one).
    isOverBudget(project: Project): boolean {
      return this.overageLines(project).length > 0;
    },
    // One text line per overage condition (can be both hours and date
    // at once) — used both in the detail popup banner and in the Play
    // confirmation message.
    overageLines(project: Project): string[] {
      if (project.closed) return [];
      const lines: string[] = [];
      const workedSeconds = this.statsFor(project).totalSeconds;

      if (project.budgetHours != null && workedSeconds > project.budgetHours * 3600) {
        const over = workedSeconds - project.budgetHours * 3600;
        lines.push(
          this.$t('projects.overBudget', {
            amount: formatHoursMinutes(over),
            hours: project.budgetHours,
          }) as string
        );
      }

      if (project.targetDate != null && moment().isAfter(moment(project.targetDate), 'day')) {
        const days = moment().startOf('day').diff(moment(project.targetDate).startOf('day'), 'days');
        const dayWord =
          days === 1 ? this.$t('projects.overDeadlineDay') : this.$t('projects.overDeadlineDays');
        lines.push(
          `${this.$t('projects.overDeadlinePrefix')} ${days} ${dayWord} ${this.$t('projects.overDeadlineExpected', { date: formatLongDate(project.targetDate) })}`
        );
      }

      return lines;
    },
    // Confirmed "start anyway" in the overage popup: run the toggle
    // that was put on hold.
    async confirmPendingPlay() {
      const project = this.pendingPlayProject;
      this.pendingPlayProject = null;
      if (project) await this.performToggle(project);
    },
    cancelPendingPlay() {
      this.pendingPlayProject = null;
    },
  },
});
