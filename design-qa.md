# Design QA — FedoraUpdate 0.2.7

## Evidence

- User empty-state reference: `/tmp/codex-clipboard-4d4c17d4-cfd6-46a2-aeac-454b9dcabeee.png`.
- User layout reference: `/tmp/codex-clipboard-e579d31c-8327-44f3-ab29-b981377781fa.png`.
- Empty-state implementation: `/home/andre/.codex/visualizations/2026/08/19/01a01a2e-197a-7f23-a910-bbf54c932f01/fedoraupdate-022-empty.png`.
- Six-update implementation: `/home/andre/.codex/visualizations/2026/08/19/01a01a2e-197a-7f23-a910-bbf54c932f01/fedoraupdate-022-list-final.png`.
- Tray-triggered checking state: `/home/andre/.codex/visualizations/2026/08/19/01a01a2e-197a-7f23-a910-bbf54c932f01/fedoraupdate-023-checking.png`.
- Automatically refreshed result: `/home/andre/.codex/visualizations/2026/08/19/01a01a2e-197a-7f23-a910-bbf54c932f01/fedoraupdate-023-complete.png`.
- Empty-state comparison: `/home/andre/.codex/visualizations/2026/08/19/01a01a2e-197a-7f23-a910-bbf54c932f01/empty-state-comparison.png`.
- Full-layout comparison: `/home/andre/.codex/visualizations/2026/08/19/01a01a2e-197a-7f23-a910-bbf54c932f01/layout-comparison.png`.
- Expanded-grid reference: `/tmp/codex-clipboard-377f0040-b22b-4a7c-845b-823c080f3f3b.png`.
- Expanded-grid implementation: `/home/andre/.codex/visualizations/2026/08/19/01a01a2e-197a-7f23-a910-bbf54c932f01/fedoraupdate-026-grid.png`.
- Expanded-grid comparison: `/home/andre/.codex/visualizations/2026/08/19/01a01a2e-197a-7f23-a910-bbf54c932f01/fedoraupdate-026-grid-comparison.png`.
- Implementation dimensions: 870 × 850 px; GTK content is configured for 820 × 800 CSS px and the capture includes client-side decorations.
- States verified: no pending updates and six pending updates, dark theme, daily verification at 10:00.

## Findings

No actionable P0, P1, or P2 mismatches remain.

- Empty-state icon: replaced the unavailable `emblem-ok-symbolic` asset with the real Adwaita `object-select-symbolic` icon. The focused comparison confirms that it renders correctly.
- Vertical sizing: the update list now grows from one through four visible rows according to the package count. Larger result sets scroll inside the list instead of creating an oversized empty card or pushing controls away.
- Fixed actions: both installation buttons and the schedule row are hosted in the `AdwToolbarView` bottom bar, outside the scrollable page.
- Tray activation: the banner is visible when either the GNOME AppIndicator extension is disabled or the tray process is absent. Its action enables the extension and starts `fedoraupdate-tray`, while reporting a useful error if activation fails.
- Cross-process feedback: a check started from the AppIndicator changes the open window to “Verificando atualizações…” and disables conflicting actions; completion reloads the cache without reopening the application.
- Tray state: the menu immediately reports the running check. Pending updates use the existing `-updates-symbolic` icon with an integrated notification dot; no numeric label is added beside the icon.
- Native attention state: pending updates switch the AppIndicator to `Attention`; the GNOME extension and active shell theme determine the final highlight color while the asset remains symbolic.
- Hybrid attention asset: the normal state remains symbolic, while pending updates use a regular hicolor icon with neutral arrows and an orange notification dot so GNOME does not flatten the dot to the panel foreground color.
- Expanded package viewport: the preferences group and its internal scroller consume all remaining height above the fixed actions. The outer page no longer scrolls; only the package rows scroll when they exceed the available viewport.
- Visual language: spacing, typography, surfaces, semantic blue action and symbolic icons remain native to libadwaita.
- Copy and accessibility: all app-specific labels are Brazilian Portuguese; controls retain native focus and disabled states.

## Primary Checks

- Empty and populated fixtures render without GTK allocation warnings after the final sizing fix.
- The fixed footer remains fully visible in both states.
- Lists above four packages expose internal scrolling while preserving the footer.
- The 15-package live cache displays seven visible rows instead of four and extends the viewport to the button spacing boundary without moving the footer.
- Rust unit tests: 3 passed.
- GTK4/libadwaita GUI check and debug build: passed on Fedora 44.
- Live `systemd --user` integration: the window observed the service in `activating`, rendered the checking state, then returned to the refreshed result when the service became inactive.
- Destructive installation actions were not invoked during visual QA.

final result: passed
