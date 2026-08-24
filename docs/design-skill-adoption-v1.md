# Design-skill adoption, version 1

## Purpose

DiskTrace uses a project-local `evidenceforge-desktop-design` skill for changes to the native eframe/egui recovery workspace. The skill was created from vetted public design-review, accessibility, and egui implementation guidance; it is not a copied web skill and contains no downloaded executable code, package, font, image, or runtime dependency.

> The interface is intentionally a restrained forensic workstation rather than a generic AI dashboard. It prioritizes task completion, evidence clarity, local-first safety, and explicit blocked states.

## Adopted guidance

| Source | Applied principle | DiskTrace adaptation |
|---|---|---|
| Microsoft frontend-design review skill | Clear primary action, quality craft, trustworthy status and error communication, and avoidance of generic interface aesthetics. [1] | Each recovery state exposes one next safe action: select an image, scan, choose a destination, or recover. |
| Addy Osmani accessibility guidance | Visible labels, focusable controls, non-color-only state, and meaningful empty, loading, and error states. [2] [3] | Recovery, validation, source-integrity, destination, and blocked-export states use explicit text alongside visual tone. |
| Official egui guidance | Cross-platform eframe support, AccessKit support, and configurable style/visual systems. [4] | The native UI uses eframe layout primitives, explicit style configuration, bounded scroll regions, and platform-native verification. |
| User-supplied interface and designer skill collections | Design-system tokens, state mapping, progressive disclosure, layout density, native interaction vocabulary, and deletion of nonessential visual competition. [5] [6] [7] | The application uses a named palette, a state-specific next-action model, adaptive evidence panes, a platform-command shortcut reference, and a single first-run primary-action location. |
| User-supplied native desktop guidance | Respect platform muscle memory and real system affordances rather than simulating web conventions. [8] | DiskTrace retains system file dialogs, native eframe controls, Command/Control shortcut semantics, and no WebView or telemetry dependency. |

## Implemented interface decisions

The product header shows local-only, read-only-source, and current workflow status. The workflow rail states the three recovery decisions and scrolls independently at shorter heights. The central first-run view presents a bounded orientation flow, while scan-complete states use evidence cards, method and validation badges, source-integrity metrics, and a focused selected-result panel.

Selected evidence has an independently scrollable detail region. The export state is intentionally explicit: a missing destination presents a separate-storage explanation and **Choose destination** as the only primary action; a source-integrity problem presents a blocked-recovery explanation without an export action. These choices preserve the existing destination policy and source verification rather than merely restyling them.

## Color-system contract

The desktop source defines a single `Palette` rather than scattering raw RGB values. Deep graphite and slate tokens separate the canvas, navigation chrome, evidence surfaces, and raised cards. Cool cyan is reserved for focus, active progression, and destination selection. Mineral green means verified or completed; amber means review or caution; coral means failure or unavailable source. Recovery-method tones remain muted identifiers and never replace validation or safety wording.

The visual system deliberately avoids glow effects, purple gradients, stock imagery, glassmorphism, and generic dashboard ornament. Every safety state continues to use descriptive text alongside color.

## Verification boundary

The local UI contract checks the visible safety labels, workflow rail, filters, scrollable evidence detail, and export-readiness guidance. Native X11 interaction checks exercise the guided scan, candidate selection, and detail scrolling at the 1440 × 920 desktop target. This does not replace native assistive-technology validation on Windows, Linux, and macOS.

## References

[1]: https://github.com/microsoft/skills/blob/main/.github/skills/frontend-design-review/SKILL.md "Microsoft: Frontend Design Review skill"
[2]: https://github.com/addyosmani/agent-skills/blob/main/skills/frontend-ui-engineering/SKILL.md "Addy Osmani: Frontend UI Engineering skill"
[3]: https://github.com/addyosmani/agent-skills/blob/main/references/accessibility-checklist.md "Addy Osmani: Accessibility checklist"
[4]: https://github.com/emilk/egui "egui and eframe project documentation"
[5]: https://github.com/jakubkrehel/skills "Better Interface Suite"
[6]: https://github.com/Owl-Listener/designer-skills "Designer Skills"
[7]: https://github.com/julianoczkowski/designer-skills "Designer Skills for Prototyping"
[8]: https://github.com/yetone/native-feel-skill "Native Feel Skill"
