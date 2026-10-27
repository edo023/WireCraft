# Guida ai comandi

Tutti i comandi si usano dai menu, dalla barra a schede, dalla riga di comando e da script o agenti AI (`cadcraft-cli`, MCP) con parametri JSON: `HELP` o l'elenco comandi mostra i parametri di ciascuno.

## Il programma

All'avvio, senza un disegno da aprire, si apre **Gestione Progetti**. In alto la barra a schede: WireCraft, Fili e Morsetti, Simboli, Strumenti, Quadro, Disegno, Vista, Output. A sinistra i pannelli Fogli, Simboli (libreria con anteprime), Navigatore, Cross e Materiali; a destra la tavolozza dei comandi; in basso la barra di stato con foglio, progetto, quadro corrente e conduttore in uso.

Finestre: Gestione Progetti, Gestione Fogli (con revisioni e fogli in libreria), Impostazioni, inserimento e modifica del simbolo (XS), Liste e Tabelle, Gestione Cavi, Gestione PLC, Traduttore, Stampa e PDF Fogli.

### Progetti e settaggi

| Comando | Cosa fa |
|---|---|
| `COMMESSA.NUOVA`, `COMMESSA.NUOVOMULTIFOGLIO`, `COMMESSA.APRI`, `COMMESSA.MODIFICA`, `COMMESSA.COPIA`, `COMMESSE.ELENCO` | gestione dei progetti | `COMMESSA.NUOVA` accetta `configurazione` per partire dalle impostazioni di un cliente. |
| `IMPOSTAZIONI` (`SETTAGGI`) | siglatura, numerazione fili, conduttori, quadro/funzione/ubicazione correnti, morsetti, nomi delle alimentazioni |
| `CONFIGURAZIONE.SALVA`, `.USA`, `.ELIMINA`, `CONFIGURAZIONI` | configurazioni con nome (una per cliente): salva le impostazioni in uso, usale in un altro progetto (quadro, funzione e ubicazione correnti restano), elenco |
| `CONFIGURAZIONE.ESPORTA`, `.IMPORTA` | impostazioni su file `.ini` e ritorno; si importano anche i settaggi di un altro CAD elettrico |

### Fogli (menu Fogli)

| Comando | Cosa fa |
|---|---|
| `NUOVOMULTIFOGLIO` | squadratura e cartiglio sul foglio 0, 10 fogli vuoti |
| `AGGIUNGIFOGLI` | N fogli dopo l'ultimo o da un numero, con incremento |
| `INSERISCIFOGLIO`, `ELIMINAFOGLI`, `SVUOTAFOGLI`, `RINOMINAFOGLIO`, `SHIFTFOGLI`, `COMPATTAFOGLI` | gestione dei fogli; riferimenti di contatti e rimandi seguono i fogli |
| `FOGLIOPRECEDENTE` (`FP`) / `FOGLIOSUCCESSIVO` (`FS`) / `CAMBIOFOGLIO` (`CF`) | navigazione tra i fogli |
| `TITOLOFOGLIO`, `DESCRIZIONECOLONNA` | titolo del foglio e descrizioni delle colonne, in due lingue |
| `NUMERAFOGLI`, `ELENCOFOGLI`, `COORDINATA` | numerazione, elenco, riferimento `foglio/colonnaRiga` di un punto |
| `COPIATRAFOGLI`, `SPOSTATRAFOGLI` | copia (con sigle nuove) o sposta oggetti su altri fogli |
| `MEMORIZZAFOGLIO`, `INSERISCIMEMORIZZATO` | fogli memorizzati in libreria, per categoria |
| `REVISIONE.CREA`, `.VEDI`, `.ANNULLA`, `.RESET`, `.ELENCO` | revisioni: archivio dei fogli e simbolo nel cartiglio |
| `STAMPAFOGLI` | PDF dei fogli scelti; **dinamico**: segnalibri e riferimenti cliccabili |

### Schema (menu Elettrico)

| Comando | Cosa fa |
|---|---|
| `FILO`, `CONNESSIONE`, `ALIMENTAZIONI`, `COLLEGAMENTI`, `VV`, `HH` | fili, nodi, linee di alimentazione, collegamenti a N fili, quick filo |
| `RIMANDIALIMENTAZIONI` / `RIMANDICOLLEGAMENTI`, `AGGIORNARIMANDI` | rimandi di partenza e arrivo con il riferimento all'altro (`///` = senza corrispondenza) |
| `SIMBOLO.*` (`SIMBOLO.KM`, `SIMBOLO.NA`…) | posa un simbolo, su un punto o con la **fence** su più fili |
| `EDITASIMBOLO` (`XS`), `CANCELLASIMBOLO`, `SPOSTASIMBOLO` | modifica, cancella e sposta mantenendo i collegamenti |
| `SIGLECOMPONENTI` | rinumera le sigle secondo la siglatura dei settaggi e aggiorna i contatti |
| `CROSSREFERENCE` | dove sta la madre di ogni contatto, specchietti, rimandi; elenca i problemi |
| `SALTA` (Vai al Collegato) | dal contatto alla madre, dalla madre ai suoi contatti, da un rimando al corrispondente: cambia foglio, inquadra e seleziona |
| `NUMERAFILI` (automatica), `NUMERAFILO` (manuale), `BLOCCAFILI`, `LISTANUMERIFILI` | numerazione dei fili |
| `CONDUTTORE` | specializza il conduttore (sezione, colore, codice) dei fili toccati |
| `MORSETTI`, `MORSETTIERA` | morsetti su una linea che attraversa i fili; morsettiera in tabella o CSV |
| `CONNETTORI` | pin di connettore (sesso M/F) su una linea che attraversa i fili; escono nelle stesse liste dei morsetti |
| `EDITAMORSETTO`, `EDITAFILO`, `EDITARIMANDO` | modifica di morsetti (con il controllo dei numeri doppi), fili (rinomina tutta la rete e la blocca) e rimandi (ricollega i riferimenti) |
| `AGGIORNALISTE` | cross reference, rimandi e tutte le liste già sui fogli |

### Liste, cavi, PLC (menu Progetto)

| Comando | Cosa fa |
|---|---|
| `USCITA {tipo}` | disegna sui fogli: `indice`, `distinta`, `morsettiere`, `cablaggio`, `cavi`, `conduttoricavi`, `io`, `funzioni`, `simboli`, `revisioni` |
| `DISTINTAMATERIALI`, `TABELLACABLAGGIO`, `MATERIALE`, `MATERIALI.*` | distinta e cablaggio in CSV, materiali dei componenti, archivio materiali |
| `CAVO.DEFINISCI`, `CAVO.POSA`, `CAVO.TOGLI`, `CAVO.ELIMINA`, `CAVI`, `CAVO.CONDUTTORI` | cavi e vene |
| `PLC.SCHEDA`, `PLC.NUMERABIT`, `PLC.COMMENTO`, `PLC.ELENCO`, `PLC.IMPORTA` | schede I/O, indirizzi, commenti, elenco I/O |
| `RETI` | le reti con i capi collegati (per script e agenti) |
| `TRADUCI`, `TRADUTTORE.FRASI`, `.AGGIUNGI`, `.IMPORTA`, `.ESPORTA` | schema bilingue dal dizionario (funzioni, titoli dei fogli, descrizioni di colonna); elenco delle frasi senza traduzione |

### Quadro (scheda e menu Quadro)

| Comando | Cosa fa |
|---|---|
| `LAYOUTQUADRO` | interno armadio su un foglio nuovo: piastra in scala con canaline, guide DIN, ingombri di componenti e morsettiere, tabella delle misure |
| `FORATURAPIASTRA` | fori di fissaggio a passo costante su guide e canaline, disegnati sul foglio e in CSV con le quote dall'angolo della piastra |

## Simboli (menu Simboli e Protezioni)

52 simboli IEC 60617 disegnati da noi, a categorie: contatti (NO, NC, scambio, ritardati, di potenza a 2 e 3 poli, del relè termico), bobine (relè, contattore, ritardate all'eccitazione e alla diseccitazione, elettrovalvola), comandi (pulsanti NO/NC, emergenza, luminoso, selettori), sensori (finecorsa, prossimità, fotocellula, pressostato, termostato), segnalazione (lampada, sirena), protezioni (fusibili, relè termico), interruttori (magnetotermici 1–4 poli, differenziali, salvamotore, sezionatore), motori (trifase, con PE, monofase), trasformatore e alimentatore, utilizzatori (resistenza, presa), componenti (resistore, condensatore, diodo), terre e masse.

- Le madri ricevono la sigla automatica secondo la siglatura dei settaggi e il quadro corrente.
- I contatti prendono la sigla della madre, dove si trova (`REG`) e i numeri dei morsetti dallo specchietto.
- Un simbolo inserito su un filo lo interrompe tra i suoi morsetti; cancellato o spostato, il filo si richiude.
