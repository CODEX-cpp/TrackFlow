import _ from 'lodash';

import { IBucket } from '~/util/interfaces';
import { defineStore } from 'pinia';
import { getClient } from '~/util/awclient';
import { useServerStore } from '~/stores/server';

function select_buckets(
  buckets: IBucket[],
  { host, type }: { host?: string; type?: string }
): string[] {
  return _.map(
    _.filter(
      buckets,
      bucket => (!type || bucket.type === type) && (!host || bucket.hostname == host)
    ),
    bucket => bucket['id']
  );
}

interface State {
  buckets: IBucket[];
}

export const useBucketsStore = defineStore('buckets', {
  state: (): State => ({
    buckets: [],
  }),

  getters: {
    hosts(state: State): string[] {
      // TODO: Include consideration of device_id UUID
      let hosts = _.uniq(_.map(state.buckets, bucket => bucket.hostname || bucket.data.hostname));

      // Sort priority:
      //   1. Host that matches the server's own hostname (the device the
      //      webui is running on) — almost always what the user wants by
      //      default in views like Alerts/Trends.
      //   2. Hosts that actually have the buckets we typically query against
      //      (window + AFK), so views don't pick a stale legacy hostname
      //      with only one orphan bucket and then immediately error out.
      //   3. Then by last_updated, newest first (legacy behavior).
      const serverStore = useServerStore();
      const selfHost = serverStore.info && serverStore.info.hostname;
      hosts = _.orderBy(
        hosts,
        [
          host => (host && host === selfHost ? 1 : 0),
          host => {
            const hasWindow = this.bucketsWindow(host).length > 0;
            const hasAfk = this.bucketsAFK(host).length > 0;
            return hasWindow && hasAfk ? 2 : hasWindow || hasAfk ? 1 : 0;
          },
          host => _.max(_.map(this.bucketsByHostname[host], b => b.last_updated)) || '',
        ],
        ['desc', 'desc', 'desc']
      );
      return hosts;
    },
    // TrackFlow is single-device (mono-client, explicit design decision) — this
    // is the one place every consumer should resolve "the" host from, instead
    // of a route param, a dropdown, or looping over `hosts`. Still backed by
    // the ranked `hosts` getter above so the right bucket set is picked even
    // if a stale/orphan hostname exists alongside the real one.
    host(): string {
      return this.hosts[0] || '';
    },

    bucketsByType(
      this: State
    ): (host: string, type: string, fallback_unknown_host?: boolean) => string[] {
      return (host, type, fallback_unknown_host) => {
        let buckets = select_buckets(this.buckets, { host, type });
        if (fallback_unknown_host && buckets.length == 0) {
          buckets = select_buckets(this.buckets, { host: 'unknown', type });
          //console.log('fallback: ', buckets);
        }
        return buckets;
      };
    },

    // Convenience getters for bucketsByType
    bucketsAFK(): (host: string) => string[] {
      return host => this.bucketsByType(host, 'afkstatus');
    },
    bucketsWindow(): (host: string) => string[] {
      return host =>
        this.bucketsByType(host, 'currentwindow').filter(
          (id: string) => !id.startsWith('aw-watcher-android')
        );
    },
    bucketsAndroid(): (host: string) => string[] {
      return host => {
        const android = this.bucketsByType(host, 'currentwindow').filter((id: string) =>
          id.startsWith('aw-watcher-android')
        );
        // aw-import-screentime uses bucket type 'app'; filter by name prefix to avoid
        // matching unrelated 'app'-type buckets from other sources.
        const ios = this.bucketsByType(host, 'app').filter((id: string) =>
          id.startsWith('aw-import-screentime')
        );
        return [...android, ...ios];
      };
    },
    bucketsEditor(): (host: string) => string[] {
      // fallback to a bucket with 'unknown' host, if one exists.
      // TODO: This needs a fix so we can get rid of this workaround.
      return host => this.bucketsByType(host, 'app.editor.activity', true);
    },
    bucketsExcel(): (host: string) => string[] {
      // fallback to a bucket with 'unknown' host, if one exists.
      // TODO: This needs a fix so we can get rid of this workaround.
      return host => this.bucketsByType(host, 'app.excel.activity', true);
    },
    bucketsVoispeed(): (host: string) => string[] {
      // fallback to a bucket with 'unknown' host, if one exists.
      // TODO: This needs a fix so we can get rid of this workaround.
      return host => this.bucketsByType(host, 'app.voispeed.calls', true);
    },
    bucketsBrowser(): (host: string) => string[] {
      // fallback to a bucket with 'unknown' host, if one exists.
      // TODO: This needs a fix so we can get rid of this workaround.
      return host => this.bucketsByType(host, 'web.tab.current', true);
    },
    bucketsStopwatch(): (host: string) => string[] {
      // fallback to a bucket with 'unknown' host, if one exists.
      // TODO: This needs a fix so we can get rid of this workaround.
      return (host: string) => this.bucketsByType(host, 'general.stopwatch', true);
    },

    getBucket(this: State): (id: string) => IBucket {
      return id => _.filter(this.buckets, b => b.id === id)[0];
    },
    bucketsByHostname(this: State): Record<string, IBucket[]> {
      return _.groupBy(this.buckets, 'hostname');
    },
  },

  actions: {
    async ensureLoaded(): Promise<void> {
      if (this.buckets.length === 0) {
        await this.loadBuckets();
      }
    },

    async loadBuckets(): Promise<void> {
      const buckets = await getClient().getBuckets();
      this.update_buckets(buckets);
    },

    async getBucketWithEvents({
      id,
      start,
      end,
      limit,
    }: {
      id: string;
      start?: Date;
      end?: Date;
      limit?: number;
    }) {
      await this.ensureLoaded();
      const bucket = _.cloneDeep(this.getBucket(id));
      bucket.events = await getClient().getEvents(bucket.id, {
        start,
        end,
        limit: limit || -1,
      });
      return bucket;
    },

    async getBucketsWithEvents({ start, end }: { start?: Date; end?: Date }) {
      await this.ensureLoaded();
      const buckets = await Promise.all(
        _.map(
          this.buckets,
          async bucket => await this.getBucketWithEvents({ id: bucket.id, start, end })
        )
      );
      return _.orderBy(buckets, [b => b.id], ['asc']);
    },

    async deleteBucket({ bucketId }: { bucketId: string }) {
      console.log(`Deleting bucket ${bucketId}`);
      await getClient().deleteBucket(bucketId);
      await this.loadBuckets();
    },

    // mutations
    update_buckets(this: State, buckets: IBucket[]): void {
      this.buckets = _.orderBy(buckets, [b => b.id], ['asc']).map(b => {
        // Some harmonization as aw-server-rust and aw-server-python APIs diverge slightly
        if (!b.last_updated && b.metadata && b.metadata.end) {
          b.last_updated = b.metadata.end;
        }
        if (!b.first_seen && b.metadata && b.metadata.start) {
          b.first_seen = b.metadata.start;
        }
        if (!b.first_seen && b.created) {
          b.first_seen = b.created;
        }

        return b;
      });
    },
  },
});
