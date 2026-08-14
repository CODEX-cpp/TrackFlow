aw-server-rust-src
===================

Codice sorgente Rust vendorizzato — datastore, motore di interrogazione
(AQL) e modelli dati. Usato da TrackFlow come librerie interne
incorporate nell'app, **non** come server standalone: niente binario
da lanciare a parte, niente porta HTTP, niente file di configurazione
da scrivere a mano.

## Cosa contiene

- `aw-server/` — datastore in-process ed endpoint di query, usati
  direttamente da `src-tauri` (vedi `src-tauri/src/lib.rs`)
- `aw-datastore/` — storage su SQLite
- `aw-models/` — tipi dati condivisi (eventi, bucket, query)
- `aw-query/` — linguaggio di interrogazione (AQL) usato sia dalle
  visualizzazioni della Home sia dai tool dell'agente AI
  (`src-tauri/src/agent.rs`)
- `aw-transform/` — funzioni di trasformazione eventi (`flood`,
  `filter_keyvals`, `merge_events_by_keys`, ...) richiamate dalle query
  AQL

## Come si compila

Queste crate sono dipendenze dirette di `src-tauri` (vedi
`src-tauri/Cargo.toml`) — la build reale è quella dell'app Tauri:

```sh
cd src-tauri
cargo build
```

Per il flusso completo di build/deploy (frontend + backend + copia del
`dist`), vedi `BLUEPRINT.md` nella cartella principale di TrackFlow.

## Licenza

Codice distribuito sotto Mozilla Public License 2.0 — vedi `LICENSE`.
