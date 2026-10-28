<p align="center"><img src="assets/logo-scritta.svg" alt="WireCraft" width="520"></p>

<h3 align="center">Il CAD elettrico open source per l'automazione industriale.</h3>

<p align="center">
Schemi multifoglio, morsettiere, cavi, PLC, distinte, lay-out del quadro e PDF.<br>
Gratis, leggero, nativo, e pronto per lavorare insieme all'intelligenza artificiale.
</p>

<p align="center">
<img alt="Licenza MIT o Apache-2.0" src="https://img.shields.io/badge/licenza-MIT%20%2F%20Apache--2.0-2ea44f">
<img alt="Rust" src="https://img.shields.io/badge/Rust-1.95%2B-b7410e?logo=rust">
<img alt="Windows" src="https://img.shields.io/badge/piattaforma-Windows-0078d6">
<img alt="Stato alpha" src="https://img.shields.io/badge/stato-alpha-orange">
<img alt="54 test di integrazione" src="https://img.shields.io/badge/test-54%20passati-brightgreen">
</p>

<p align="center"><img src="docs/img/schema.png" alt="WireCraft: lo schema di potenza di un motore, con i fogli del progetto a sinistra e i comandi elettrici a destra" width="100%"></p>

---

## Perché WireCraft

Chi progetta quadri elettrici oggi paga licenze costose, ogni anno, per programmi chiusi, pesanti e legati a una piattaforma CAD di terzi. I progetti restano prigionieri di formati e librerie che non si possono portare altrove.

**WireCraft è l'alternativa libera.** Lo installi, lo usi, lo modifichi, lo integri: senza licenze, senza abbonamenti, senza chiedere il permesso a nessuno.

- ⚡ **Nativo e leggero**: scritto in Rust, parte in un attimo e gira anche su PC modesti.
- 🧭 **Flusso di lavoro professionale**: commesse, multifoglio, siglatura, numerazione fili, cross reference, morsettiere, cavi, PLC, liste. Chi arriva da un CAD elettrico tradizionale si ritrova subito a casa.
- 🔓 **Aperto davvero**: progetti in DXF con i dati elettrici negli attributi, liste in CSV, PDF per tutti. I progetti restano tuoi.
- 🤖 **Pronto per l'AI**: ogni comando ha parametri JSON, quindi script e agenti AI possono disegnare, numerare e verificare uno schema come farebbe un progettista.
- 🔁 **Compatibile**: apre le commesse esistenti in formato `Job-Description._sp` e i loro DWG, con fogli, titoli, rimandi, fili e contatti. Su un progetto reale di 114 rimandi ricalcola tutti i riferimenti incrociati, senza un errore.

## Cosa sa fare

<table>
<tr>
<td width="50%" valign="top">

**📁 Progetti e fogli**
- commesse con l'elenco dei multifogli e i settaggi di commessa
- multifoglio A3 con squadratura 0–9 × A–F e cartiglio compilato da solo
- aggiungi, inserisci, elimina, rinomina, sposta e compatta i fogli: i riferimenti seguono
- titoli e descrizioni di colonna in due lingue
- fogli memorizzati (pagine standard in libreria) e revisioni
- configurazioni con nome per cliente, riusabili nei progetti nuovi

**⚡ Schema**
- fili ortogonali, nodi automatici, alimentazioni e collegamenti a N fili
- rimandi tra fogli con riferimento `foglio/colonnaRiga`
- 52 simboli IEC 60617 disegnati da zero, con anteprime
- inserimento su più fili in un colpo solo (fence); cancella e sposta richiudono i fili
- specchietti dei contatti sotto bobine, relè e salvamotori
- fili colorati come il conduttore vero, dalla tabella delle tensioni (L1 marrone, PE giallo-verde, +24V blu…)

**🏷️ Automatismi**
- siglatura dei componenti in sei modalità, per quadro
- numerazione fili automatica e manuale, fili bloccati, numeri doppi segnalati
- cross reference dinamico con i problemi in rosso, giallo e verde
- navigatore ad albero: quadri, componenti, alimentazioni, rimandi, figli orfani
- vai al collegato: dal contatto alla bobina, dalla bobina ai contatti, tra rimandi
- designazione CEI EN 81346: box di quadro, funzione e ubicazione, etichette sopra la sigla

</td>
<td width="50%" valign="top">

**🔌 Morsetti, connettori, cavi, PLC**
- morsettiere posate con una linea attraverso i fili, con ponticelli a mano o automatici
- connettori con pin maschio/femmina
- cavi con le vene posate sui fili e i segnalini
- schede I/O PLC con indirizzi e commenti in due lingue
- configuratore macchina: opzioni che si escludono, progetto generato con le sole opzioni scelte
- loop diagram: per ogni ingresso e uscita il percorso fino al dispositivo in campo

**📋 Liste e uscite**
- sui fogli, sempre aggiornabili: lista fogli, distinta materiali, morsettiere, tabella di cablaggio, lista cavi, lista I/O, legende di funzioni, simboli e revisioni
- tutto anche in CSV ed Excel
- archivio materiali con importazione del listino
- targhette di componenti, fili, morsetti e cavi in CSV per la stampante

**🌍 Traduttore**
- dizionario in cinque lingue (IT, EN, DE, FR, ES)
- schema bilingue con un comando: funzioni, titoli, colonne

**🗄️ Quadro**
- layout del quadro: piastra in scala con canaline, guide DIN e ingombri dei componenti presi dallo schema
- foratura della piastra con le quote, in CSV
- calcolo termico: temperatura interna, ventilatore o condizionatore che servono, anticondensa

**🖨️ Stampa**
- PDF dinamico: segnalibri dei fogli e riferimenti cliccabili
- fili a colori e tutto il resto in nero, come una stampa da consegnare

</td>
</tr>
</table>

### Il lay-out del quadro nasce dallo schema

I componenti del quadro e le morsettiere finiscono sulle guide DIN in scala, tra le canaline. La piastra si sceglie da sola tra le misure commerciali, e la foratura esce con le quote.

<p align="center"><img src="docs/img/layout-quadro.png" alt="Il lay-out dell'interno del quadro generato dallo schema, con la tabella degli ingombri" width="100%"></p>

### Le uscite sono già pronte da consegnare

Schema con i fili colorati come i conduttori, calcolo termico del quadro, loop diagram e tabella di cablaggio: generati dal progetto, bilingui, nel PDF con i rimandi cliccabili.

<p align="center"><img src="docs/img/output-pdf.png" alt="Quattro pagine del PDF: schema di un motore a fili colorati, calcolo termico, loop diagram, tabella di cablaggio" width="100%"></p>

## Provalo

Servono [Rust](https://rustup.rs) 1.95 o più recente e, su Windows, i Build Tools di Visual Studio (C++).

```sh
git clone --recurse-submodules https://github.com/edo023/WireCraft.git
cd WireCraft
scripts/aggiorna-cadcraft.sh          # prepara il motore CAD con i ganci di WireCraft
cargo run --release -p wirecraft      # avvia il programma
```

Vuoi vedere subito un progetto completo? Questo comando genera un impianto di esempio da 16 fogli (alimentazione, motore, ausiliari, PLC, lay-out del quadro, calcolo termico, loop diagram, liste e legende) e lo stampa in PDF:

```sh
cargo run --release -p elettrico --example esempio_completo -- esempio.dxf esempio.pdf
```

> Su Windows conviene compilare fuori da OneDrive: `CARGO_TARGET_DIR=C:/dev/wirecraft-target`.

Tutti i comandi, con i parametri, sono nella [guida ai comandi](docs/COMANDI.md).

## Roadmap

Dall'idea alla prima versione pubblica in cinque giorni. La storia completa, giorno per giorno, è nel [CHANGELOG](CHANGELOG.md).

| Quando | Traguardo | |
|---|---|---|
| 23 ottobre 2026 | L'idea: un CAD elettrico libero, con il flusso di lavoro dei programmi professionali | ✅ |
| 24 ottobre | Scelta del motore CAD e prova di fattibilità: blocchi con attributi, estrazione dati, DXF | ✅ |
| 25 ottobre | Modello dati e compatibilità con le commesse esistenti | ✅ |
| 26 ottobre | **Prima versione**: progetti, multifoglio, fili, simboli, siglatura, numerazione, morsetti, cavi, PLC, liste, PDF | ✅ |
| 26 ottobre | Navigatore, Cross Reference, modifica di fili, morsetti e rimandi | ✅ |
| 27 ottobre | Traduttore, lay-out del quadro, foratura piastra, connettori. **Pubblicazione open source** | ✅ |
| 27 ottobre (sera) | Vai al Collegato, configurazioni con nome per cliente, box e designazioni CEI EN 81346, calcolo termico del quadro, loop diagram, tensioni e fili colorati, targhette, ponticelli, liste in Excel, configuratore macchina; lettura migliore dei DWG di altri programmi | ✅ |
| Prossimi passi | Gestore topografico e pulsantiere | 🔜 |
| | Schema unifilare per i quadri di distribuzione | 🔜 |
| | Editor di morsettiera avanzato (ordine, multilivello) | 🔜 |
| | Installer per Windows e release con l'eseguibile pronto | 🔜 |

## Com'è fatto

WireCraft è un livello elettrico sopra [CADCraft](https://github.com/storytold/cadcraft), un CAD 2D/3D open source in Rust (MIT/Apache-2.0) con comandi in stile classico, blocchi con attributi e file DXF/DWG. WireCraft non lo modifica: lo include come submodule e aggiunge i suoi comandi dall'esterno, così ogni miglioramento di CADCraft arriva anche qui.

| Cartella | Contenuto |
|---|---|
| `apps/wirecraft` | il programma desktop (`wirecraft.exe`): barra a schede, pannelli, finestre |
| `crates/elettrico` | tutti i comandi elettrici, con 54 test di integrazione in `tests/integrazione` |
| `crates/elettrico/examples` | `esempio_completo` (un progetto con tutte le funzioni), `multifoglio`, `analizza` (blocchi e attributi di un DWG) |
| `vendor/cadcraft` | CADCraft, come git submodule |
| `patches/cadcraft` | i piccoli ganci che servono a CADCraft (comandi e menu di un'applicazione) e la versione su cui si applicano |
| `scripts/aggiorna-cadcraft.sh` | prepara CADCraft dopo un clone, o lo porta all'ultima versione (`--ultima`) |

## Contribuire

WireCraft è nato per essere costruito da chi lo usa: progettisti elettrici, quadristi, integratori, sviluppatori.

- **Prova il programma** e apri una issue con quello che manca o non funziona.
- **Simboli e librerie**: nuovi simboli IEC, cartigli, fogli memorizzati.
- **Codice**: ogni funzione nuova arriva con il suo test in `crates/elettrico/tests/integrazione`.

## Licenza

WireCraft è distribuito, a tua scelta, con licenza [MIT](LICENSE-MIT) o [Apache-2.0](LICENSE-APACHE), come CADCraft.

WireCraft è un progetto indipendente. Codice, simboli, grafica e testi sono originali; i formati di file di altri programmi si leggono e si scrivono solo per l'interoperabilità.
