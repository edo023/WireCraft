# Storia di WireCraft

Il progetto giorno per giorno: cosa è stato fatto e cosa è stato verificato. Le voci più recenti sono in alto.

## Prossimi passi

- **Loop diagram**: il percorso di ogni utenza in campo (sensore → morsetti → PLC), generato su fogli nuovi.
- **Configuratore macchina**: opzioni con codici e regole di esclusione; lo schema si genera con le sole opzioni scelte.
- **Gestore topografico e pulsantiere**: posizione dei componenti nell'impianto, fronte e retro delle pulsantiere.
- **Calcolo delle sovratemperature** del quadro dalle potenze dissipate.
- **Schema unifilare** per i quadri di distribuzione, con la tabella dati di ogni circuito.
- **CEI EN 81346**: box di quadro e di funzione che assegnano l'appartenenza ai componenti.
- **Estrazione targhette** per sigle, fili e morsetti.
- **Esportazione Excel** delle liste; editor di morsettiera avanzato (ponticelli, multilivello).
- **Installer per Windows** e release con l'eseguibile pronto.

### Problemi noti

- Nei DWG creati da altri programmi CADCraft riempie i contorni "riquadro di testo" dei tratteggi e disegna i testi delle tabelle (`TABLE`) con l'altezza dello stile invece di quella delle celle. Da correggere in CADCraft.

## 27 ottobre 2026: traduttore, quadro, connettori, pubblicazione

- **Traduttore**: dizionario CSV in cinque lingue (IT, EN, DE, FR, ES) con le frasi frequenti degli schemi di macchine e quadri. Il comando `TRADUCI` rende bilingue lo schema (funzioni dei componenti, titoli dei fogli, descrizioni di colonna) ed elenca le frasi senza traduzione. Una frase scritta in maiuscolo resta in maiuscolo.
- **Layout quadro** (`LAYOUTQUADRO`): su un foglio nuovo la piastra in scala, con canaline ai lati e tra le file, guide DIN e gli ingombri dei componenti del quadro presi dallo schema, morsettiere comprese. La piastra si sceglie tra le misure commerciali; una tabella elenca componenti e misure.
- **Foratura piastra** (`FORATURAPIASTRA`): fori di fissaggio a passo costante su guide e canaline, anche se spostate a mano; disegnati sul foglio e in CSV con le quote dall'angolo della piastra.
- **Connettori**: pin con sesso maschio/femmina, posati come i morsetti; si modificano come i morsetti, ricevono i numeri dei fili ed escono nelle stesse liste.
- **Pubblicazione open source** con licenza MIT o Apache-2.0.

Verificato: gli ingombri del lay-out non diventano componenti dello schema; la traduzione copre funzioni, titoli e colonne; i fili numerati arrivano sui pin dei connettori.

## 26 ottobre 2026 (sera): navigatore, cross reference, modifica

- **Navigatore** ad albero: quadri, componenti, alimentazioni, rimandi, figli orfani; un clic porta al simbolo.
- **Pannello Cross Reference** con i problemi in rosso, giallo e verde.
- **Barra di stato** con la scelta del foglio a tendina.
- **Modifica** dalla selezione: `EDITAMORSETTO` (controlla i numeri doppi), `EDITAFILO` (rinomina tutta la rete e la blocca), `EDITARIMANDO` (ricollega i riferimenti).

Verificato: rinominare un rimando aggiorna la coppia di partenza e arrivo e segnala quelli rimasti senza corrispondenza.

## 26 ottobre 2026: la prima versione

Il programma `wirecraft.exe` con logo e icona, la barra a schede e i comandi elettrici:

- **progetti**: una cartella per commessa con l'elenco dei multifogli, compatibile con il formato `Job-Description._sp`; nuovo progetto, nuovo multifoglio con cartiglio compilato, apri, copia, settaggi per commessa;
- **multifoglio**: aggiungi, inserisci, elimina, svuota, rinomina, shift, compatta, copia e sposta tra fogli, fogli memorizzati, revisioni, descrizioni di colonna;
- **fili, alimentazioni, collegamenti e rimandi** con i riferimenti tra fogli;
- **libreria di 52 simboli IEC** con specchietti dei contatti e testine aggiuntive, inserimento su fence, cancella e sposta che richiudono i fili;
- **siglatura** nelle sei modalità, **numerazione fili** automatica e manuale, conduttori, morsetti, cavi, schede PLC;
- **liste e tabelle** aggiornabili sui fogli, CSV, **PDF dinamico**.

Verificato:
- il multifoglio e i fili sopravvivono al salvataggio e alla riapertura del DXF;
- un solo Annulla toglie un multifoglio intero;
- ogni simbolo della libreria si inserisce, interrompe i fili sotto ai poli e li richiude quando si cancella o si sposta;
- inserire, eliminare, spostare e compattare i fogli aggiorna i riferimenti di contatti e rimandi;
- su un progetto reale esistente (114 rimandi) i riferimenti incrociati di rimandi e contatti ricalcolati sono tutti uguali a quelli originali.

## 25 ottobre 2026: modello dati

- Studio dei formati delle commesse e dei DWG esistenti (nomi dei blocchi, attributi, regole), per leggere e scrivere progetti compatibili.

## 24 ottobre 2026: il motore CAD

- Scelta di [CADCraft](https://github.com/storytold/cadcraft) (Rust, MIT/Apache-2.0) come base, incluso come submodule e mai modificato a mano.
- Prova di fattibilità: definizione di attributi, creazione di blocchi, inserimento con attributi, estrazione dati in CSV, salvataggio in DXF e PNG. Funziona tutto.
- Ganci minimi per i comandi e i menu di un'applicazione, in `patches/cadcraft`.

## 23 ottobre 2026: l'idea

Un CAD elettrico per l'automazione industriale **open source, leggero e scritto bene**, con il flusso di lavoro dei programmi professionali: fogli, simboli, componenti, morsettiere, cavi e report, tutto al suo posto.
