# Il sito di WireCraft

Il sito vive nella cartella `sito/`: HTML, CSS e JavaScript statici, senza build e senza risorse esterne. Si pubblica su Cloudflare Pages all'indirizzo `https://wirecraft.alfredopennisi.com`.

## Pubblicare su Cloudflare Pages

1. Cloudflare → **Workers & Pages** → **Create** → **Pages** → **Connect to Git** → repository `edo023/WireCraft`.
2. Impostazioni di build:
   - Production branch: `main`
   - Framework preset: *None*
   - Build command: *(vuoto)*
   - Build output directory: `sito`
3. **Save and Deploy**. Da qui ogni push su `main` ripubblica il sito.
4. Nel progetto Pages → **Custom domains** → **Set up a custom domain** → `wirecraft.alfredopennisi.com`. Se il dominio è già su Cloudflare, il record DNS si crea da solo.

In alternativa, senza Git: `npx wrangler pages deploy sito --project-name wirecraft`.

## Dopo la prima pubblicazione

- **Email**: Cloudflare → dominio → **Email** → **Email Routing**: crea l'indirizzo `wirecraft@alfredopennisi.com` e inoltralo alla tua casella. È il contatto scritto in privacy e note legali.
- **Statistiche senza cookie**: progetto Pages → **Metrics** → abilita **Web Analytics**. L'informativa privacy le prevede già.
- **Bot AI**: dominio → **Security** → **Bots**: lascia **disattivati** "Block AI bots" e il "robots.txt gestito", altrimenti ChatGPT, Claude e Perplexity non possono leggere e citare il sito.
- **Crawler Hints**: dominio → **Caching** → **Configuration** → attiva **Crawler Hints** (avvisa Bing e Yandex dei cambiamenti con IndexNow).

## Indicizzazione

1. **Google Search Console** (search.google.com/search-console): aggiungi la proprietà *Dominio* `alfredopennisi.com` (verifica con il record TXT, che Cloudflare aggiunge in un clic) e invia `https://wirecraft.alfredopennisi.com/sitemap.xml`. Poi **Controllo URL** → home → **Richiedi indicizzazione**, anche per `/en/`.
2. **Bing Webmaster Tools** (bing.com/webmasters): importa la proprietà da Search Console. Bing alimenta anche ChatGPT Search e Copilot.
3. **GitHub**, nella pagina del repository → ⚙️ accanto a *About*:
   - Website: `https://wirecraft.alfredopennisi.com`
   - Description: `CAD elettrico open source per schemi, quadri e automazione industriale, pronto per l'IA`
   - Topics: `electrical-cad`, `cad`, `electrical-engineering`, `schematic`, `industrial-automation`, `plc`, `rust`, `ai`, `mcp`, `iec-60617`, `open-source`, `cad-elettrico`
4. **Link in entrata**: sono il fattore che conta di più per salire. Metti il link al sito nel README, nelle note di ogni release, sul profilo GitHub e LinkedIn, sul sito principale `alfredopennisi.com`. Presenta il progetto dove si parla di automazione e di open source (forum di elettrotecnica, gruppi LinkedIn di quadristi, r/PLC, r/ElectricalEngineering, Hacker News "Show HN", elenchi "awesome" di software CAD libero).

## File del sito

| File | A cosa serve |
|---|---|
| `index.html`, `en/index.html` | pagina italiana e inglese, con `hreflang`, Open Graph e dati strutturati (schema.org) |
| `privacy/`, `note-legali/` | informativa GDPR e cookie; licenze, marchi, interoperabilità, limiti di responsabilità |
| `robots.txt`, `sitemap.xml` | istruzioni per i motori di ricerca, crawler AI ammessi |
| `llms.txt` | riassunto del progetto per gli assistenti AI |
| `_headers` | intestazioni di sicurezza (CSP, HSTS), cache, `noindex` sull'indirizzo `*.pages.dev` |
| `img/` | screenshot in WebP, icone, immagine di anteprima per i social (`og-image.png`) |

Le immagini nascono da `docs/img/*.png` e `assets/logo.svg`. Quando cambiano gli screenshot, rigenera i WebP alle stesse dimensioni e aggiorna `lastmod` in `sitemap.xml`.

## Regole per i testi

- Nessun nome di altri CAD elettrici o dei loro produttori, nemmeno nei confronti o nelle parole chiave; niente "alternativa a…", "clone", "compatibile con…" seguito da un marchio.
- Le funzioni descritte devono esistere nell'ultima release. Quello che non c'è ancora va scritto come "in arrivo".
- Niente risorse esterne (font, script, iframe): il sito resta senza banner dei cookie.
