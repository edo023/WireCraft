# Storia di WireCraft

Il progetto giorno per giorno: cosa è stato fatto e cosa è stato verificato. Le voci più recenti sono in alto.

## Prossimi passi

- **Gestore topografico**: posizione dei componenti nell'impianto.
- **Editor di morsettiera** avanzato (ordine, multilivello).
- **Installer per Windows** e release con l'eseguibile pronto.

## 28 ottobre 2026: schema unifilare

- **Schema unifilare dei quadri di distribuzione** (`UNIFILARE.ARRIVO`, `UNIFILARE.CIRCUITO`, `UNIFILARE.MODIFICA`, `UNIFILARE.AGGIORNA`): un circuito per colonna appeso alle sbarre, con protezione (magnetotermico, magnetotermico differenziale, differenziale, sezionatore, fusibili), cavo con i tratti dei conduttori, utenza (prese, illuminazione, motore, sottoquadro, riscaldamento, generica, riserva) e sotto la tabella dei dati: circuito, denominazione in due lingue, sistema, tensione, potenza, cos φ, Ib, curva, In, Icn, Idn e tipo, cavo, formazione, lunghezza, Iz, caduta di tensione, verifica. L'arrivo linea sta sopra le sbarre nella prima colonna; un quadro continua sui fogli successivi.
- **Dimensionamento**: corrente d'impiego dalla potenza; taratura normalizzata subito sopra; sezione minima con la portata giusta (IEC 60364-5-52, cavi multipolari in rame EPR/XLPE, posa B2, C o E, fattore di correzione) e la caduta di tensione dall'origine entro il limite; in automatico l'arrivo si tiene un quarto della caduta ammessa. Taratura e sezione restano automatiche finché non si scrivono a mano, e si rifanno a ogni ricalcolo.
- **Verifica**: Ib ≤ In ≤ Iz (con i fusibili In ≤ 0,9 Iz), caduta di tensione totale, potere di interruzione rispetto al cortocircuito presunto dell'arrivo. L'arrivo prende la potenza delle partenze per il coefficiente di contemporaneità.
- La protezione è un componente come gli altri: sigla automatica, codice e costruttore in distinta materiali, descrizione dai dati (`Magnetotermico differenziale 2P C16 Idn 30 mA A`) se non se ne sceglie una dall'archivio.
- **Tabella dei circuiti** (`USCITA {tipo: circuiti}`, anche in Excel); Aggiorna Liste e Tabelle ricalcola anche l'unifilare.
- Finestra **Schema Unifilare** (scheda Quadro, gruppo Distribuzione): elenco dei circuiti con la verifica in verde o rosso, modulo per disegnare un circuito o l'arrivo, scelta di un circuito dall'elenco per modificarlo.
- Il progetto d'esempio ha un foglio in più: il quadro di distribuzione del reparto che alimenta la macchina.

Verificato: 10 kW trifase su 50 m passano da 1,5 a 2,5 mm² per la caduta di tensione; le prese non scendono sotto 2,5 mm²; cambiata la protezione, il blocco cambia e i dati restano; cancellato un circuito, Aggiorna Liste toglie utenza, conduttori, sbarre e intestazioni del foglio rimasto vuoto.

## 27 ottobre 2026 (sera): dopo la pubblicazione

- **Vai al Collegato** (`SALTA`): dal contatto alla sua madre, dalla madre ai suoi contatti uno dopo l'altro, da un rimando a quello corrispondente; cambia foglio, inquadra e seleziona, così ripetendo si torna indietro.
- **Configurazioni con nome**: le impostazioni (siglatura, numerazione fili, conduttori, morsetti, alimentazioni) si salvano con un nome, per cliente o per tipo di lavoro, e si riusano in un altro progetto o alla creazione di un progetto nuovo. Esportazione e importazione su file `.ini`, anche dai settaggi di un altro CAD elettrico. Nella finestra Impostazioni, sezione Configurazioni.
- **CEI EN 81346**: ogni componente ha funzione (`=`), ubicazione (`++`) e quadro (`+`); i simboli nuovi prendono quelli correnti. I **box** di quadro, funzione e ubicazione (riquadri tratto-punto) li assegnano a tutto quello che racchiudono; le **etichette di designazione** li mostrano sopra la sigla, anche solo dove il box non li dice già. Nella scheda Quadro della barra.
- **Sovratemperature** (`CALCOLOTERMICO`): bilancio termico del quadro con la superficie efficace secondo l'installazione (IEC/TR 60890); temperatura interna, portata del ventilatore o potenza del condizionatore che servono, resistenza anticondensa. Potenza dei componenti dal campo "Potenza dissipata" della finestra del simbolo, o stimata per tipo. Finestra dedicata e resoconto sul foglio, aggiornabile.
- **Loop diagram** (`USCITA {tipo: loop}`, `LOOPDIAGRAM`): per ogni ingresso e uscita dei PLC il percorso fino al dispositivo in campo, attraverso morsetti, cavi e rimandi su altri fogli, con i numeri dei fili e l'altro capo del dispositivo; i bit collegati direttamente a una linea di alimentazione risultano "potenziale +24V". Fogli a tre zone (campo, quadro, PLC) o tabella, aggiornabili.
- **Tensioni e fili colorati**: tabella dei potenziali con tensione e colore del conduttore (IEC 60757; predefiniti da CEI EN 60445 e 60204-1). Le reti senza conduttore specializzato prendono il colore del loro potenziale, anche nella tabella di cablaggio; `COLORAFILI` colora i fili del disegno; la stampa "fili colorati" tiene i fili a colori e mette in nero tutto il resto.
- **Estrazione targhette** (`TARGHETTE`): componenti (sigla e funzione), fili (una per capo collegato), morsetti (con la testata della morsettiera), cavi (una per estremità), per quadro e in ordine naturale, in CSV per la stampante. Scheda Output, gruppo Targhette.
- **Lettura dei DWG di altri programmi** (patch a CADCraft in `patches/cadcraft`): i contorni "riquadro di testo" dei tratteggi non si riempiono più (nei cartigli comparivano rettangoli pieni sopra i campi), e le tabelle riprendono l'altezza dei testi delle celle dal disegno della tabella stessa (prima uscivano enormi e sovrapposte).
- **Ponticelli** tra i morsetti (`PONTICELLO`, `PONTICELLI.AUTO`): a mano su un elenco di morsetti (`1-4, 7`) o automatici tra morsetti vicini sullo stesso filo (N, PE, +24V distribuiti); gruppo nell'attributo `PONTE`, colonna nella lista delle morsettiere e nel CSV. Nella tavolozza, sezione Morsetti.
- **Liste in Excel** (`EXCEL {tipo, file}`): ogni lista (distinta, morsettiere, cablaggio, cavi, I/O, loop, calcolo termico, legende) in un file `.xlsx` con titolo, intestazioni e larghezze di colonna; i numeri dei fili restano testo. Pulsante "Excel…" nella finestra Liste e Tabelle.
- **Configuratore macchina** (`CONFIGURATORE`, `.ABBINA`, `.ELIMINA`, `.GENERA`): opzioni con codice, descrizione e gruppo di esclusione; le parti dello schema si abbinano dalla selezione; il progetto si genera in un nuovo file con le sole opzioni scelte (le parti comuni restano sempre). Le opzioni sono gruppi con nome del disegno e si salvano nel file. Finestra Configuratore nella scheda Strumenti.
- **Pulsantiera** (`PULSANTIERA`, `USCITA {tipo: pulsantiera}`): pulsanti, emergenze, selettori, lampade e sirene del quadro disposti in file sulla porta (emergenze per prime, file centrate), disegnati in scala con sigla e tipo (lampade col colore della funzione) ed elencati con funzioni e quote; foratura Ø22,5 in CSV ed Excel. Pulsante nella scheda Quadro.
- Etichette di menu, barra a schede e finestre riscritte con parole nostre; comandi e scorciatoie invariati.

Verificato: il salto dal contatto porta alla bobina sul suo foglio e ritorno; i contatti di una madre escono in ordine di foglio; un rimando porta al suo corrispondente, uno senza coppia non porta da nessuna parte. Una configurazione usata in un progetto non ne cambia il quadro corrente; un nome sbagliato non lascia un progetto creato a metà. Fra box dello stesso tipo uno dentro l'altro vale il più piccolo; un componente spostato in un altro box prende il nuovo quadro con Aggiorna Liste e Tabelle.

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
