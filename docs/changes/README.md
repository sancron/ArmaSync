# Änderungsjournal

Jeder Task schreibt eine eigene Datei `YYYY-MM-DD-MIG-NNN.md`, auf Basis von [TEMPLATE.md](TEMPLATE.md). So ändern parallele Systeme nicht dieselbe Journaldatei. Änderungen erklären Zweck, Verhalten, Dateien, Prüfungen, Migration-/Branding-/Lizenzfolgen und Risiken. Nach Integration bleibt die Datei als Historie erhalten; Korrekturen transparent ergänzen.

[LAST_CHANGE.md](LAST_CHANGE.md) verweist ausschließlich auf die zuletzt **integrierte** Änderung. Nur der Integrator aktualisiert sie; Git-Mergereihenfolge ist maßgeblich, nicht Dateizeit oder Chatabschluss. `CHANGELOG.md` im Root bietet die nutzerorientierte Releaseübersicht.
