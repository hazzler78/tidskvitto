# Tidskvitto

En dag mer frihet. Med AI.

Tidskvitto är en skrivbordsapp för macOS och Windows från 4 Days AI AB. Den mäter tyst hur en kunskapsarbetare använder arbetsdagen: vilken app och vilket fönster som är aktivt, hur länge, och hur stor del som går till e-post. Resultatet är ett tidskvitto med en ärlig uppskattning av hur mycket en AI-agent skulle kunna ta över.

Appen läser inte mejl, filer eller tangenttryckningar. Allt lagras lokalt tills användaren själv kopierar en sammanfattning eller slår på en egen API-nyckel.

## Vad appen gör

1. **Mätning.** Ett bakgrundsspår tar ett prov ungefär varannan sekund: appnamn, fönstertitel och om datorn verkar stå still. Tomgång (standard tre minuter utan inmatning) räknas inte som arbetstid. Tidskvittos eget fönster räknas inte heller.
2. **Kvitto.** Dag och vecka (måndag–söndag) med tid per kategori och app, andel e-post, andel koordinering och en uppskattning. Veckan följer datorns tidszon.
3. **Chatt.** Frågor som "Vad tog mest tid förra veckan?" och "Hur mycket mejlade jag i tisdags?" besvaras lokalt utan nyckel. Med en nyckel skickas bara en sammanfattning till den leverantör användaren valt.
4. **Förslag.** Om samma e-postfönster återkommer, eller om e-post och koordinering tar mycket tid, visas ett lokalt förslag. Appen skickar inga mejl och påstår inte att den läst dem.

### Kategorier

Heuristiken ligger i `crates/core` och är medvetet enkel:

| Kategori | Exempel |
| --- | --- |
| Fokusarbete | Editorer, Office, webbläsare som inte är mejl eller möte |
| E-post | Outlook, Apple Mail, Thunderbird och webbmejl (Gmail, Outlook på webben) igenkänt på fönstertiteln |
| Koordinering | Slack, Teams-chatt, kalender, andra chattappar |
| Möten | Zoom, Teams-möte, Google Meet, FaceTime |
| Övrigt | Allt annat, till exempel musik |

Webbmejl känns igen på titeln, inte på adressen. Appen frågar inte webbläsaren om URL.

### Uppskattningen

Varje kategori multipliceras med ett antagande som går att ändra i inställningarna. Standard:

- e-post 40 %
- koordinering 20 %
- fokusarbete 15 %
- möten 10 %
- övrigt 0 %

Taket är 80 % per kategori. Summan är tid en agent *skulle kunna* ta över, inte tid som redan sparats. Standardvärdet för e-post ligger i intervallet 30–50 % som 4days.ai brukar nämna för repetitiva uppgifter. Det är ett antagande, inte ett resultat från loggen och inte ett resultat från andras piloter. Formeln står på kvittot.

## Integritet

- Databasen ligger i appens datamapp (`tidskvitto.db`). Sökvägen visas i inställningarna.
- Ingenting skickas förrän användaren kopierar sammanfattningen eller ställer en fråga med en sparad nyckel.
- Till AI:n går summerade tider, andelar och antaganden. Fönstertitlar ingår bara om användaren slår på det. Mejladresser i titeln byts då mot `[mejl]`.
- API-nyckeln sparas i systemets nyckelring när den finns, annars i den lokala databasen. Den skickas bara till basadressen i inställningarna. Adressen måste vara `https`, utom `localhost`.
- Mätningen startar inte förrän användaren godkänt den första rutan. Den pausas med reglaget eller från menyraden. Att stänga fönstret pausar inte; appen ligger kvar tills man väljer Avsluta.
- "Radera all mätdata" tömmer segmenten. "Radera allt" tar även inställningar och nyckel.

## Teknikval

Tauri 2 med ett React-gränssnitt och en Rust-kärna (`tidskvitto-core`).

Mätningen är ett bakgrundsjobb. Rust kan läsa aktivt fönster och tomgång utan att dra in en hel Chromium-process, och SQLite ligger i samma process som kvittot. Gränssnittet delas mellan macOS och Windows via systemets WebView (WebKit respektive WebView2). Electron hade gjort det lättare att hitta ett npm-paket för aktivt fönster, men hade kostat en webbläsare för en app som mest ska ligga i menyraden.

Aktivt fönster kommer från biblioteket `x-win` (appnamn och titel). Vi anropar inte dess funktion för webbläsaradresser. Tomgång läses via `GetLastInputInfo` på Windows och `CGEventSource` på macOS.

Logiken för kategorier, midnatt, uppskattning och de lokala svaren är enhetstestad utan fönster.

## Köra och bygga

Kräver Node.js 22, Rust 1.100 (filen `rust-toolchain.toml` väljer den) och Tauri-förutsättningarna för respektive system: [tauri.app/start/prerequisites](https://v2.tauri.app/start/prerequisites/).

```bash
npm ci
npm test          # Rust-tester för kvittot
npm run dev       # bara gränssnittet, med exempeldata, på http://127.0.0.1:1420
npm run tauri dev # appen, med riktig mätning
npm run tauri build -- --bundles app   # macOS: .app
npm run tauri build -- --no-bundle     # Windows: exe i src-tauri/target/release
```

### macOS

Bygget ad-hoc-signeras (`signingIdentity: "-"`). Det är inte ett Apple-certifikat. Nedladdade testbyggen stoppas ofta av Gatekeeper:

1. Högerklicka på `Tidskvitto.app` och välj Öppna, sedan Öppna igen.
2. Om det inte räcker: `xattr -dr com.apple.quarantine Tidskvitto.app`

Skärminspelning behövs för fönstertitlar, och därmed för att känna igen Gmail och annan webbmejl. macOS kallar behörigheten skärminspelning även när appen inte tar några bilder.

1. Öppna Systeminställningar.
2. Gå till Integritet och säkerhet → Skärm- och systemljudsinspelning.
3. Aktivera Tidskvitto. Saknas den, lägg till den från Programmappen.
4. Starta om Tidskvitto.

Appnamn kan mätas utan den behörigheten. Appen ber inte om tillgänglighet.

### Windows

Ingen extra behörighet behövs för aktivt fönster. WebView2 ingår i Windows 10 och 11. Ett osignerat testbygge kan varnas av SmartScreen: Mer information → Kör ändå. Det finns inget utgivarcertifikat i det här repot, och inget ska skapas.

### Linux

Inte en målplattform. Skalet kompilerar för utveckling. Tomgång kräver `xprintidle`. Wayland/GNOME kan behöva mer än X11 för fönstertitlar; då mäts bara det `x-win` faktiskt kan se.

## CI

GitHub Actions (`.github/workflows/ci.yml`) kör testerna, typkollar gränssnittet, kompilerar skalet på Linux och laddar upp osignerade testbyggen för macOS (`.app`) och Windows (`.exe`). Inga releaser publiceras.

## Kända begränsningar

- Bara det främsta fönstret. Flera skärmar och bakgrundsmöten syns inte.
- Tomgång är senaste tangent eller mus, inte om personen faktiskt arbetar. Ett möte där man sitter still kan bli tomgång efter tröskeln. Om tomgång inte kan läsas räknas all tid som aktiv, och det sägs i inställningarna.
- Runt omställning till sommar- eller vintertid kan en timme hamna på fel dygn, eftersom dygnet räknas i den tidszon datorn har just då.
- Teams-möte mot Teams-chatt, och webbmejl mot en vanlig flik, är gissningar utifrån appnamn och titel. En Word-fil som heter "Mötesanteckningar" räknas som fokus, inte möte.
- Tid då appen inte körs mäts inte. Stänga fönstret lämnar mätningen igång; Avsluta stoppar den.
- Chatten sparas inte på disk.
- Delegering är ett förslag i gränssnittet. Appen tar inte över mejl.
- Förhandsvisningen i webbläsaren (`npm run dev`) använder exempeldata och anropar ingen AI-leverantör.
- Ingen telefonapp, ingen automatisk uppdatering, ingen riktig kodsignering.

## Projektet

```
crates/core     kategorier, databas, kvitto, lokal chatt
src-tauri       fönster, menyrad, mättråd, AI-anrop
src             svenskt gränssnitt
```

4 Days AI AB · 4days.ai · En dag mer frihet. Med AI.
