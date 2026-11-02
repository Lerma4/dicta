# Dicta nativa per macOS — anteprima

Questa versione SwiftUI/AppKit è in migrazione e non sostituisce ancora l'app Tauri. Richiede macOS 13 o successivo, Command Line Tools e CMake per la compilazione.

```sh
bash macos/Scripts/build-app.sh
open macos/.build/Dicta.app
```

In Impostazioni concedere Microfono e Accessibilità. In Modelli selezionare un modello installato o scaricarne uno esplicitamente. Con la finestra Dicta aperta, selezionare un campo di testo in un'altra app e usare ⌘⇧Spazio per iniziare e terminare. Se l'inserimento nel campo originale non è possibile, recuperare il testo in Dicta con la copia esplicita. Per Rex l'inserimento usa temporaneamente gli appunti e ⌘V, senza inviare Invio; i gestori di appunti possono vedere il testo. Dicta ripristina gli appunti precedenti quando non sono stati modificati nel frattempo. L'audio resta in memoria e l'inferenza usa Whisper.cpp localmente.

La versione nativa registra statistiche essenziali delle dettature completate in `~/Library/Application Support/app.dicta.native/sessions.sqlite3`. La cronologia testuale è facoltativa, disattivata inizialmente e salvata in chiaro sul Mac; disattivarla interrompe i nuovi salvataggi senza eliminare i testi già presenti. Il pulsante nella cronologia li elimina mantenendo le statistiche. La vista mostra fino a 200 trascrizioni recenti.

La build usa Accelerate e disattiva esplicitamente OpenMP: la libreria Homebrew disponibile su macOS 26 richiede macOS 26, mentre Dicta dichiara il supporto da macOS 13. Non è stato misurato l'impatto sulle prestazioni.

**Limiti:** il percorso microfono → trascrizione → inserimento e i permessi non sono stati verificati end-to-end. La scorciatoia è attiva solo mentre la finestra è aperta. Le statistiche e la cronologia della versione Tauri non vengono ancora importate; il database precedente non viene modificato. Grafici, filtri, esportazione, overlay e impostazioni complete non sono ancora migrati. L'inferenza utilizza CPU, non Metal. L'app è firmata ad hoc, non notarizzata e non è pronta per la distribuzione.

Verifiche disponibili: `sh macos/Scripts/test-core.sh`. `swift test --package-path macos` richiede una toolchain con XCTest.
