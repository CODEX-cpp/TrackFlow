export interface IEvent {
  timestamp: string;
  duration: number;
  data: Record<string, any>;
}

export interface IBucket {
  id: string;
  hostname: string;
  device_id: string;
  type: string;
  // Presente nella risposta reale del server (vedi aw-models Bucket
  // struct) ma finora mai serviva lato webui — usato ora dal pannello
  // "Stato watcher" per distinguere i bucket dei watcher riconosciuti
  // da quelli esterni/personalizzati (client non in KNOWN_CLIENTS).
  client?: string;
  data: Record<string, any>;
  metadata?: { start: Date; end: Date };
  last_updated?: Date;
  first_seen?: Date;
  created?: Date;
}
