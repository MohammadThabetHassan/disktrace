# Candidate Catalogue v1

The candidate catalogue converts recovery-engine output into deterministic, user-facing results without changing the source image or altering recovery evidence. It accepts a fixed list of candidates and produces a sorted list, summary counts, optional text search, method filtering, validation filtering, and plain-language explanations.

Search is case-insensitive and matches candidate identifier, evidence name, file type, recovery method, and validation state. Filters are additive: a candidate must meet every selected filter. The default order is source offset ascending, then candidate identifier ascending. Stable ordering is part of the public output contract so the desktop interface and reports can reproduce a result selection exactly.

Preview metadata is descriptive and bounded. A preview descriptor may expose a safe file type, byte length, source range, and a limited text excerpt only for recovered UTF-8 text content. It must never execute content, render embedded HTML, inspect macros, invoke external programs, or modify the source. Binary candidates receive metadata-only previews until a sandboxed renderer is introduced.

Plain-language explanations must state the recovery method and what it does not prove. A FAT12 metadata candidate explains that directory metadata and a readable cluster chain were used, while noting that deleted metadata can be incomplete. A PNG carved candidate explains that file structure was found in raw storage bytes and that the original filename/folder is unavailable. Validation labels are not percentages: `content_validated` means the supported structural checks passed, while `recovered_unvalidated` means bytes were recovered but no content validator established completeness.
