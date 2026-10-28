# Guida ai comandi

Tutti i comandi si usano dai menu, dalla barra a schede, dalla riga di comando e da script o agenti AI (`cadcraft-cli`, MCP) con parametri JSON: `HELP` o l'elenco comandi mostra i parametri di ciascuno.

## Il programma

All'avvio, senza un disegno da aprire, si apre **Gestione Progetti**. In alto la barra a schede: WireCraft, Fili e Morsetti, Simboli, Strumenti, Quadro, Disegno, Vista, Output. A sinistra i pannelli Fogli, Simboli (libreria con anteprime), Navigatore, Cross e Materiali; a destra la tavolozza dei comandi; in basso la barra di stato con foglio, progetto, quadro corrente e conduttore in uso.

Finestre: Gestione Progetti, Gestione Fogli (con revisioni e fogli in libreria), Impostazioni, inserimento e modifica del simbolo (XS), Liste e Tabelle, Gestione Cavi, Gestione PLC, Traduttore, Stampa e PDF Fogli.

### Progetti e settaggi

| Comando | Cosa fa |
|---|---|
| `COMMESSA.NUOVA`, `COMMESSA.NUOVOMULTIFOGLIO`, `COMMESSA.APRI`, `COMMESSA.MODIFICA`, `COMMESSA.COPIA`, `COMMESSE.ELENCO` | gestione dei progetti | `COMMESSA.NUOVA` accetta `configurazione` per partire dalle impostazioni di un cliente. |
| `COMMESSA.ESEMPIO` | progetto d'esempio | Crea la commessa "Esempio WireCraft" con un progetto completo (19 fogli), la salva e la apre. Pulsante "Progetto d'esempio" in Gestione Progetti. |
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
| `STAMPAFOGLI` | PDF dei fogli scelti; **dinamico**: segnalibri e riferimenti cliccabili; `filiColorati`: i fili a colori e il resto in nero |

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
| `TENSIONI` | la tabella dei potenziali: tensione e colore del conduttore (`BK`, `BN`, `BU`, `GNYE`…); le reti senza indicazione sul filo prendono il colore del loro potenziale |
| `COLORAFILI` | i fili del disegno col colore del conduttore (`togli`: di nuovo col colore del layer) |
| `CONDUTTORE` | specializza il conduttore (sezione, colore, codice) dei fili toccati |
| `MORSETTI`, `MORSETTIERA` | morsetti su una linea che attraversa i fili; morsettiera in tabella o CSV |
| `PONTICELLO`, `PONTICELLI.AUTO` | ponticelli tra morsetti della stessa morsettiera: a mano (`morsetti: "1-4, 7"`, `togli`) o automatici tra morsetti vicini sullo stesso filo |
| `CONNETTORI` | pin di connettore (sesso M/F) su una linea che attraversa i fili; escono nelle stesse liste dei morsetti |
| `EDITAMORSETTO`, `EDITAFILO`, `EDITARIMANDO` | modifica di morsetti (con il controllo dei numeri doppi), fili (rinomina tutta la rete e la blocca) e rimandi (ricollega i riferimenti) |
| `AGGIORNALISTE` | cross reference, rimandi e tutte le liste già sui fogli |

### Liste, cavi, PLC (menu Progetto)

| Comando | Cosa fa |
|---|---|
| `USCITA {tipo}` | disegna sui fogli: `indice`, `distinta`, `morsettiere`, `cablaggio`, `cavi`, `conduttoricavi`, `io`, `funzioni`, `simboli`, `revisioni`, `termico`, `loop`, `pulsantiera` (con `tabella: true` in tabella) |
| `DISTINTAMATERIALI`, `TABELLACABLAGGIO`, `MATERIALE`, `MATERIALI.*` | distinta e cablaggio in CSV, materiali dei componenti, archivio materiali |
| `CAVO.DEFINISCI`, `CAVO.POSA`, `CAVO.TOGLI`, `CAVO.ELIMINA`, `CAVI`, `CAVO.CONDUTTORI` | cavi e vene |
| `PLC.SCHEDA`, `PLC.NUMERABIT`, `PLC.COMMENTO`, `PLC.ELENCO`, `PLC.IMPORTA` | schede I/O, indirizzi, commenti, elenco I/O |
| `RETI` | le reti con i capi collegati (per script e agenti) |
| `EXCEL {tipo, file}` | la lista `tipo` (come per `USCITA`) in un file Excel `.xlsx` |
| `TARGHETTE {tipo}` | targhette in CSV: `componenti`, `fili` (una per capo), `morsetti` (con la testata), `cavi` (una per estremità); `quadro` per filtrare |
| `CONFIGURATORE`, `.ABBINA`, `.ELIMINA`, `.GENERA` | opzioni della macchina (codice, descrizione, gruppo di esclusione), parti abbinate (dalla selezione), progetto generato in un nuovo file con le sole opzioni scelte |
| `LOOPDIAGRAM` | per ogni bit dei PLC: fili, morsetti, cavi, dispositivo in campo e suoi altri capi (sui fogli: `USCITA {tipo: loop}`) |
| `TRADUCI`, `TRADUTTORE.FRASI`, `.AGGIUNGI`, `.IMPORTA`, `.ESPORTA` | schema bilingue dal dizionario (funzioni, titoli dei fogli, descrizioni di colonna); elenco delle frasi senza traduzione |

### Quadro (scheda e menu Quadro)

| Comando | Cosa fa |
|---|---|
| `LAYOUTQUADRO` | interno armadio su un foglio nuovo: piastra in scala con canaline, guide DIN, ingombri di componenti e morsettiere, tabella delle misure |
| `BOX.QUADRO`, `BOX.FUNZIONE`, `BOX.UBICAZIONE` (`BOX {tipo}`) | riquadro tratto-punto che dà quadro (`+QG`), funzione (`=TR1`) o ubicazione (`++ZC`) secondo la CEI EN 81346 a madri, morsetti e connettori che racchiude; fra box dello stesso tipo vale il più piccolo |
| `BOX.APPLICA` | riapplica i box dopo aver spostato qualcosa (lo fa anche `AGGIORNALISTE`) |
| `DESIGNAZIONI` | etichette sopra la sigla con i livelli scelti (`=TR1++ZC+QG`), anche sui contatti con i dati della madre; regola `discordanti`: solo quello che il box non dice già; `togli` le cancella |
| `CALCOLOTERMICO` (Sovratemperature) | bilancio termico del quadro: superficie efficace secondo l'installazione (1–7), materiale, temperature; temperatura interna, ventilatore (m³/h) o condizionatore (W), anticondensa. Misure dell'armadio dalla piastra del lay-out; potenze dal campo `DISSIPAZIONE` dei componenti o stimate. Sul foglio con `USCITA {tipo: termico}` |
| `PULSANTIERA` | i dispositivi di comando e segnalazione del quadro disposti sulla porta, con la foratura (CSV con `file`); sul foglio con `USCITA {tipo: pulsantiera}` |
| `UNIFILARE.ARRIVO` | l'arrivo linea del quadro (`quadro`) nella prima colonna del foglio corrente: protezione generale sopra le sbarre; `icc` (kA presunti), `kc` (contemporaneità), `cdtMax` (%, default 4) |
| `UNIFILARE.CIRCUITO` | una partenza nella prossima colonna libera (14 per foglio): `utenza` (`prese`, `luce`, `motore`, `quadro`, `riscaldamento`, `generica`, `riserva`), `protezione` (`mt`, `mtd`, `id`, `sez`, `fus`), `sistema` (`3F+N+PE`, `F+N+PE`…), `potenza` (kW), `cosfi`, `lunghezza` (m), `cavo`, `posa` (`B2`, `C`, `E`), `k`, `denominazione`, `denominazione2`, `curva`, `icn`, `idn`, `classe`, `codice`, `costruttore`; `in` e `sezione` se non date si dimensionano e restano automatiche |
| `UNIFILARE.MODIFICA` | cambia i dati di un circuito (`handle`); `dimensiona: true` rende di nuovo automatiche taratura e sezione non date; un'altra `protezione` cambia il blocco |
| `UNIFILARE.AGGIORNA` | ricalcola tutti i circuiti (l'arrivo dalle sue partenze) e ridisegna sbarre, conduttori e utenze; lo fa anche `AGGIORNALISTE` |
| `UNIFILARE` | elenco dei circuiti (`quadro` per filtrare) con Ib, In, Iz, caduta di tensione e verifica; sui fogli e in Excel con `USCITA {tipo: circuiti}` |
| `FORATURAPIASTRA` | fori di fissaggio a passo costante su guide e canaline, disegnati sul foglio e in CSV con le quote dall'angolo della piastra |

## Simboli (menu Simboli e Protezioni)

52 simboli IEC 60617 disegnati da noi, a categorie: contatti (NO, NC, scambio, ritardati, di potenza a 2 e 3 poli, del relè termico), bobine (relè, contattore, ritardate all'eccitazione e alla diseccitazione, elettrovalvola), comandi (pulsanti NO/NC, emergenza, luminoso, selettori), sensori (finecorsa, prossimità, fotocellula, pressostato, termostato), segnalazione (lampada, sirena), protezioni (fusibili, relè termico), interruttori (magnetotermici 1–4 poli, differenziali, salvamotore, sezionatore), motori (trifase, con PE, monofase), trasformatore e alimentatore, utilizzatori (resistenza, presa), componenti (resistore, condensatore, diodo), terre e masse.

- Le madri ricevono la sigla automatica secondo la siglatura dei settaggi e il quadro corrente.
- I contatti prendono la sigla della madre, dove si trova (`REG`) e i numeri dei morsetti dallo specchietto.
- Un simbolo inserito su un filo lo interrompe tra i suoi morsetti; cancellato o spostato, il filo si richiude.
