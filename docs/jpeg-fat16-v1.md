# JPEG Carving and FAT16 v1 Contract

## JPEG carving

The first JPEG carver recognizes a JPEG start-of-image marker and requires a structurally plausible marker sequence that includes a Start of Frame marker before it accepts a candidate. It scans forward to the first end-of-image marker within a fixed upper bound. The carver deliberately treats that boundary as a conservative candidate boundary, not proof that all original bytes were contiguous or that the original filename and folder are known.

A candidate is labelled as signature carving. It receives a generated evidence name, source byte offset, byte length, and a validation state that identifies structural checks completed by the carver. The initial fixture includes one complete JPEG byte sequence outside filesystem metadata and one malformed marker sequence that must be ignored.

## FAT16 metadata recovery

FAT16 support reuses the controlled short-name root-directory workflow already implemented for FAT12. The parser accepts only 512-byte sectors, one FAT copy, fixed-size root directories, one sector per cluster, and a standard data-region layout. It rejects extended partitions, long file names, subdirectories, multi-sector clusters, damaged FAT chains, and images whose cluster count does not classify as FAT16.

Deleted root-directory entries are preserved as evidence records. Recovery follows the retained FAT16 cluster chain only when it is complete enough for the advertised file size. The deleted leading filename character remains unavailable and is represented by a question mark rather than guessed.

## Ground truth

The deterministic fixture generator creates one FAT16 image containing a recoverable deleted text file and a raw, structurally valid JPEG sequence that has no directory entry. Verification must confirm metadata recovery, JPEG carving, distinct method labels, safe export, receipts, and destination-policy enforcement.
