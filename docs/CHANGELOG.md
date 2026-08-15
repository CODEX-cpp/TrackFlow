# Changelog

Ogni versione ha una sezione `### it` e una `### en` — l'app mostra solo
quella della lingua attiva, per la versione in esecuzione (vedi
`src-tauri/src/about.rs`, che scarica questo file da GitHub Pages).
Non toccare questo formato senza aggiornare anche quel parser.

## 0.1.10

### it
- Corretto un bug per cui i moduli "Applicazioni principali" e "Titoli finestra principali" nella Home potevano mostrare "Nessun dato" tornando da un'altra pagina, pur essendoci dati reali.
- Corretto un piccolo difetto grafico in Impostazioni → Informazioni (titolo troppo vicino al riquadro sottostante).

### en
- Fixed a bug where the "Top Applications" and "Top Window Titles" Home modules could show "No data" after navigating back from another page, even with real data available.
- Fixed a minor visual glitch in Settings → About (section title sitting too close to the box below it).
