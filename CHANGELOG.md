# Storia di WireCraft

Il progetto giorno per giorno: cosa è stato fatto e cosa è stato verificato. Le voci più recenti sono in alto.

## Prossimi passi

- **Firma digitale** dell'eseguibile e dell'installatore.
- **Simbologie** oltre la IEC 60617.

## 30 ottobre 2026: prova su un progetto reale

Tutte le uscite e i comandi nuovi provati su un progetto esistente di 50 fogli (9.578 entità, 128 componenti, 187 morsetti), salvato e riaperto senza perdite. Corretto quello che è venuto fuori:

- **Schemi di altri programmi**: i simboli che non sono della libreria ora hanno i loro pin (l'estremo di filo accanto all'attributo `PINA1`, `PINB1`…), e i morsetti che interrompono il filo (i fili arrivano sopra e sotto) si collegano alle reti. Su quel progetto la tabella di cablaggio passa da 0 righe a 7 fogli, e il disegno delle morsettiere trova i collegamenti di 127 morsetti su 187 (prima nessuno). Un test lo riproduce con blocchi costruiti apposta.
- **Velocità**: le reti leggevano i componenti una volta per foglio; ora una volta sola. Aggiorna Liste e Tabelle su 108 fogli scende da 49 a 17 secondi; loop diagram da 8,7 a 2,0 s, cablaggio da 6,9 a 2,9 s.
- **Lay-out**: le protezioni dello schema unifilare occupano i moduli dei loro poli (F+N due, 3F+N quattro), non sempre tre.
- **Schema Unifilare**: la finestra si apre sul quadro dei circuiti del foglio corrente, o su tutti; prima filtrava sul quadro corrente e poteva sembrare vuota.
- **Liste e Tabelle**: il campo della morsettiera c'è anche per il disegno delle morsettiere.
- **Fronte quadro** (pulsante "Fronte Quadro", `PULSANTIERA.DISEGNA`, `.POSA`, `.AGGIORNA`): prima la porta era un unico disegno rifatto da Aggiorna Liste, con i dispositivi in fila per sigla che non si potevano spostare. Ora la porta sta su un suo foglio e ogni pulsante, lampada o selettore è un blocco a sé: lo si sposta a mano e resta lì, anche disegnando di nuovo o aggiornando le liste; i dispositivi nuovi dello schema vanno nei posti liberi. Ognuno segue sigla e funzione del suo componente; quelli rimasti senza componente si segnalano e si tolgono. Foratura ed elenco (`USCITA {tipo: pulsantiera}`, Excel) si leggono dalle posizioni vere. La disposizione proposta mette per prime emergenze e selettori, poi pulsanti e lampade.
- **Ordine nel lay-out del quadro**: prima il sezionatore generale, poi protezioni, fusibili e contattori di potenza vicino ai loro salvamotori, poi alimentazione, PLC e relè.
- **Componenti dal codice**: gli articoli dell'archivio hanno anche il simbolo con cui si disegnano e la larghezza sulla guida DIN (colonne `Simbolo` e `Larghezza`, riconosciute anche nei listini). La **libreria di base** (circa 80 articoli generici: magnetotermici, differenziali, salvamotori, sezionatori, portafusibili, contattori, relè, temporizzatori, trasformatori, alimentatori, pulsanti, selettori, lampade, sensori) riempie l'archivio al primo avvio e si aggiunge con un pulsante. Dalla scheda Materiali, **Inserisci** posa il simbolo giusto con costruttore, codice e descrizione (un punto per componente) e **Associa** dà il materiale al componente selezionato. Il codice di un componente, posato, associato o cambiato con Modifica Simbolo, porta la sua larghezza al componente, e il lay-out del quadro la usa al posto della misura stimata.

## 29 ottobre 2026: schema d'impianto, planimetria, versione 0.3.0

- **Versione 0.3.0** pronta per la release, con l'**installatore per Windows** (Inno Setup, compilato dalla CI insieme allo zip): installazione per l'utente senza permessi di amministratore, collegamento nel menu Start, icona sul desktop a scelta, disinstallazione. Se l'installatore non si compila, la release esce comunque con lo zip.

- **Schema d'impianto** (`USCITA {tipo: impianto}`, pulsante "Schema d'impianto" nella scheda WireCraft): i quadri del progetto come blocchi, con la descrizione del box del quadro e il numero di componenti, collegati dai cavi con sigla, formazione, lunghezza e funzione. Il percorso di un cavo viene dalla sua definizione o, se manca, dai componenti a cui arrivano le sue vene. Le tratte corte stanno vicino ai quadri e quelle lunghe all'esterno, senza incrociarsi; la funzione si scrive se c'è posto. Anche in Excel; nel progetto d'esempio (22 fogli), con il cavo che dal quadro di distribuzione alimenta il quadro macchina.
- **Planimetria** (`PLANIMETRIA`, `.NUOVO`, `.POSA`, `.LUNGHEZZE`; pulsante "Planimetria"): i quadri posati sulla pianta dell'impianto in scala (anche uno dopo l'altro, Invio = il prossimo) e la lunghezza dei cavi tra quadri posati calcolata dal percorso ortogonale, più le risalite e la scorta, scritta nei cavi (anche solo dove manca); i percorsi si disegnano sulla pianta con le sigle e le lunghezze.
- Al primo avvio la sola cartella dei progetti è `Documenti\WireCraft\Commesse`: le cartelle delle commesse di altri programmi non si cercano più da sole, si aggiungono da Gestione Progetti → Cartelle dei progetti.
- Il pulsante "Simbologia" apre la libreria dei simboli IEC 60617. Nessun pulsante della barra è più spento.

## 28 ottobre 2026: schema unifilare, prima release, morsettiere, topografico

- **WireCraft 0.2.0**, la prima release: `wirecraft-windows-x64.zip` con l'eseguibile pronto, compilato e provato da GitHub Actions a ogni tag `v*`, con le note della versione.

- **Schema unifilare dei quadri di distribuzione** (`UNIFILARE.ARRIVO`, `UNIFILARE.CIRCUITO`, `UNIFILARE.MODIFICA`, `UNIFILARE.AGGIORNA`): un circuito per colonna appeso alle sbarre, con protezione (magnetotermico, magnetotermico differenziale, differenziale, sezionatore, fusibili), cavo con i tratti dei conduttori, utenza (prese, illuminazione, motore, sottoquadro, riscaldamento, generica, riserva) e sotto la tabella dei dati: circuito, denominazione in due lingue, sistema, tensione, potenza, cos φ, Ib, curva, In, Icn, Idn e tipo, cavo, formazione, lunghezza, Iz, caduta di tensione, verifica. L'arrivo linea sta sopra le sbarre nella prima colonna; un quadro continua sui fogli successivi.
- **Dimensionamento**: corrente d'impiego dalla potenza; taratura normalizzata subito sopra; sezione minima con la portata giusta (IEC 60364-5-52, cavi multipolari in rame EPR/XLPE, posa B2, C o E, fattore di correzione) e la caduta di tensione dall'origine entro il limite; in automatico l'arrivo si tiene un quarto della caduta ammessa. Taratura e sezione restano automatiche finché non si scrivono a mano, e si rifanno a ogni ricalcolo.
- **Verifica**: Ib ≤ In ≤ Iz (con i fusibili In ≤ 0,9 Iz), caduta di tensione totale, potere di interruzione rispetto al cortocircuito presunto dell'arrivo. L'arrivo prende la potenza delle partenze per il coefficiente di contemporaneità.
- La protezione è un componente come gli altri: sigla automatica, codice e costruttore in distinta materiali, descrizione dai dati (`Magnetotermico differenziale 2P C16 Idn 30 mA A`) se non se ne sceglie una dall'archivio.
- **Tabella dei circuiti** (`USCITA {tipo: circuiti}`, anche in Excel); Aggiorna Liste e Tabelle ricalcola anche l'unifilare.
- Finestra **Schema Unifilare** (scheda Quadro, gruppo Distribuzione): elenco dei circuiti con la verifica in verde o rosso, modulo per disegnare un circuito o l'arrivo, scelta di un circuito dall'elenco per modificarlo.
- Il progetto d'esempio ha un foglio in più: il quadro di distribuzione del reparto che alimenta la macchina.
- **Progetto d'esempio dall'app** (`COMMESSA.ESEMPIO`): il pulsante "Progetto d'esempio" in Gestione Progetti crea e apre la commessa "Esempio WireCraft" con lo stesso progetto completo, così chi scarica il programma ha subito qualcosa da guardare. Un test lo crea, lo salva, lo riapre e controlla che fogli, componenti, rimandi, materiali, circuiti e liste siano gli stessi, anche dopo Aggiorna Liste e Tabelle.
- **Editor Morsettiere** (pulsante Morsettiere): elenco delle morsettiere e dei connettori, morsetti in ordine con livello, filo, ponte e posizione, "Vai" al morsetto nello schema; ordine a mano (su, giù) e rinumerazione secondo lo schema, i fili o compattata, dal numero e con il passo voluti (`MORSETTIERA.ORDINE`); ponticelli; rinomina e unione (`MORSETTIERA.RINOMINA`); CSV. I livelli di un morsetto multiplo restano insieme, `PE` e `N` restano dove sono.
- Corretto: Modifica Morsetto non permetteva di mettere al livello 2 un morsetto con lo stesso numero di un altro (il controllo dei doppioni guardava il livello vecchio).
- **Disegno delle morsettiere** (`USCITA {tipo: disegnomorsettiere}`, anche dall'Editor Morsettiere): un morsetto per colonna con i collegamenti interni ed esterni (trovati seguendo i fili dal morsetto verso l'alto e verso il basso: componente e pin, altro morsetto o rimando), il numero del filo, il livello, i ponticelli e il PE in verde; più pagine per le morsettiere lunghe; la stessa cosa in Excel. Nel progetto d'esempio, che arriva a 21 fogli.
- **Gestore Topografico** (`TOPOGRAFICO`, `.POSA`, `.AGGIORNA`; pulsante nella scheda Quadro): i componenti del quadro da montare nel lay-out con lo stato (posato, da posare, non più nello schema), la posa del singolo ingombro o di tutti i mancanti uno dopo l'altro, agganciati alla guida DIN più vicina. Ogni ingombro ricorda il suo componente: se nello schema cambia sigla o codice, il lay-out segue (anche dopo aver salvato e riaperto); se il componente sparisce, l'ingombro viene segnalato e si può togliere. Le misure stimate sono indicate.
- **Test di robustezza**: ogni comando con parametri vuoti o sbagliati dà un errore, mai un arresto del programma.
- I file delle commesse si scrivono con le barre dei percorsi singole, come nei file d'origine (prima venivano raddoppiate).

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
