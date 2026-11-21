# JSA Channels - app desktop Windows / Mac (21/11/2026)

Finestra dedicata con la dashboard JSA (music.jsadjsteam.com), senza browser. Leggera: usa il motore
web gia' presente nel sistema (WebView2 su Windows, WebKit su Mac).

## Perche'
Su Mac/Safari la dashboard non puo' memorizzare la cartella dei download (Safari non lo permette) e su
Chrome a ogni riapertura bisogna riconfermare il permesso. Nell'app i brani li salva l'app stessa:
**Impostazioni > Sfoglia** una volta sola, la cartella resta memorizzata per sempre.

## Installazione
- **Windows**: `JSA-Channels-Windows-Setup.exe` (installazione per l'utente, non chiede password amministratore).
- **Mac**: `JSA-Channels-Mac.dmg` -> trascinare JSA Channels in Applicazioni.
  La prima volta: tasto destro sull'app > Apri > Apri (app non firmata Apple). Su macOS 15:
  Impostazioni di Sistema > Privacy e sicurezza > "Apri comunque".

## Come funziona
- All'avvio apre `https://music.jsadjsteam.com/`. Quando si aggiorna il sito, l'app e' gia' aggiornata.
- L'indirizzo si puo' cambiare in `settings.json` (campo `url`, es. `http://127.0.0.1:PORTA/` sul PC di casa):
  - Windows: `%APPDATA%\com.jsadjsteam.channels\settings.json`
  - Mac: `~/Library/Application Support/com.jsadjsteam.channels/settings.json`
- Lo stesso file contiene la cartella scelta (`folder`).
- Anche gli altri download della pagina (link, APK...) finiscono nella cartella scelta.
- Dashboard: blocco `jsaapp-20261121` in `app/legacy/index.html` (copia `index.html.bak-20261121-jsaapp`).
  Nell'app la cartella si presenta come una cartella "Sfoglia" di Chrome, quindi download singoli, carrello e
  download multipli funzionano come prima. Nel browser normale non cambia nulla; su Safari ora compare la
  nota che spiega perche' i brani vanno in Download.

## Libreria: segno "gia' nella mia libreria" automatico (v1.1)
- All'avvio e ogni 30 minuti l'app controlla l'XML della libreria e, se e' cambiato, la dashboard lo rimanda al motore
  (/api/rbindex, con la sessione e l'identita' gia' scelte in "Chi sei?").
- Quale XML: quello scelto con Impostazioni > "Carica XML" (es. export di rekordbox), altrimenti da solo quello di
  Musica/iTunes: `~/Music/Music/Library.xml`, `~/Music/iTunes/iTunes Music Library.xml` o `iTunes Library.xml`.
  In Musica va attivato una volta: Musica > Impostazioni > File > "Condividi XML della libreria con altre app".
- rekordbox non riscrive l'XML da solo: va riesportato (File > Esporta la collezione in formato xml, stesso nome),
  oppure resta l'agente `per-utenti/Installa-Indice-Rekordbox-Mac.command` che legge direttamente la collezione.
- Dashboard: blocco `appxml-20261121` (copia `index.html.bak-20261121-appxml`).

## Ricompilare
- **Windows** (fatto dal cloud con cargo-xwin): `npx tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc`
  oppure su un PC Windows con Rust + Node: `npm install` e `npx tauri build`.
- **Mac**: copiare la cartella `JSA-App` su un Mac e fare doppio clic su `Crea-App-Mac.command`
  (installa da solo gli strumenti; il .dmg esce sulla Scrivania e va bene per Mac Intel e Apple Silicon).
- Versione: campo `version` in `src-tauri/tauri.conf.json` e `src-tauri/Cargo.toml`.

## File
- `src-tauri/src/lib.rs` - comandi: pick_folder, get_folder, file_exists, save_file, open_folder, app_info
- `src-tauri/capabilities/dashboard.json` - quali indirizzi possono usare i comandi
- `src-tauri/tauri.conf.json` - nome, icone, installatori
- La vecchia `Windows-App-JSA` (solo Windows, server locale) resta com'era.
