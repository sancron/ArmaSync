# MIG-032R — Resume mit `Range` und Validatorvertrag (geplant, nicht begonnen)

- Status: **geplant** (2026-10-25). Eigener Umfang, getrennt von MIG-032B. Noch kein Eigentümer zugewiesen.
- Ziel: Ein unterbrochener Download einer Datei kann mit `Range` fortgesetzt werden, aber nur, wenn der Server nachweislich dieselbe Datei ausliefert.

## Warum ein eigener Vertrag nötig ist

* Die Ports sagen heute ausdrücklich: „keine stillen Wiederholungen oder Resume (MIG-032)“ (ADR-0007). Resume ändert die Port-Semantik und braucht deshalb einen eigenen ADR-Zusatz vor der Implementierung.
* Fortsetzen ist nur sicher, wenn die bereits geladenen Bytes zur **aktuellen** Revision gehören. Ein Teil aus einer alten Revision darf nie mit einem neuen Rest zusammengesetzt werden (Datenintegrität).

## Validatorvertrag (Entwurf, zu entscheiden)

1. **Identität:** Ein Teilstand wird nur fortgesetzt, wenn Repository-URL, relativer Pfad, erwartete Gesamtgröße, Hashart und erwarteter Hash der Datei aus dem Manifest dieselben sind wie beim Start.
2. **Server-Validator:** Nur mit einem vom Server gelieferten Validator (`ETag` oder `Last-Modified`), der beim Start gespeichert und bei jeder Fortsetzung über `If-Range` gesendet wird. Ohne Validator: kein Resume.
3. **Antwortprüfung:** `206 Partial Content` mit `Content-Range` (Start = Länge des Teilstands, Gesamtgröße = erwartet). Antwortet der Server `200` oder weicht etwas ab: Teilstand verwerfen, neu laden (kein stilles Anhängen).
4. **Endprüfung:** Die gesamte Datei wird wie bisher über Größe und Hash geprüft; ein Resume ersetzt die Hashprüfung nicht.
5. **Speicherort:** Teilstand in der Staging-Datei neben dem Ziel, mit dem reservierten Präfix; alte Reste nach dem Aufräumalter entfernt.

## Abhängigkeiten und Plattform

* Keine neue Crate nötig (HTTP-Header über `ureq`).
* Verhalten gegen echte Server (ETag-Semantik, `If-Range`) **nicht belegt**; nur Loopback-Server und synthetische Antworten.

## Nicht im Scope

Authentifizierung (ausgesetzt, siehe MIG-032B), parallele Teilbereiche, Linux-Läufe.
