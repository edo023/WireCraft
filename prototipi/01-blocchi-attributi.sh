#!/usr/bin/env bash
# Prova di fattibilita: blocco simbolo con attributi -> inserimento -> estrazione CSV (distinta).
# Uso: CADCLI=/c/dev/cadeo-target/release/cadcraft-cli.exe ./01-blocchi-attributi.sh OUTDIR
set -euo pipefail
CLI="${CADCLI:-cadcraft-cli}"; OUT="${1:-.}"
"$CLI" run --metric \
 --cmd 'line {"points":[[0,0],[10,0],[10,6],[0,6]],"closed":true}' \
 --cmd 'attdef {"tag":"SIGLA","at":[1,4],"height":1.5}' \
 --cmd 'attdef {"tag":"CODICE","at":[1,2],"height":1,"invisible":true}' \
 --cmd 'attdef {"tag":"DESCR","at":[1,0.5],"height":1,"invisible":true}' \
 --cmd '-block {"name":"CONTATTORE","base":[0,0],"handles":["100","101","102","103","104","105","106"]}' \
 --cmd 'insert {"name":"CONTATTORE","at":[50,50],"attribs":{"SIGLA":"KM1","CODICE":"LC1D09P7","DESCR":"Contattore 9A 230V"}}' \
 --cmd 'insert {"name":"CONTATTORE","at":[80,50],"attribs":{"SIGLA":"KM2","CODICE":"LC1D09P7","DESCR":"Contattore 9A 230V"}}' \
 --cmd "dataextraction {\"blocks\":\"CONTATTORE\",\"combine\":false,\"file\":\"$OUT/bom.csv\"}" \
 --save "$OUT/test.dxf" --export "$OUT/test.png"
