# PNG Carving v1 Recovery Boundary

DiskTrace v0.1 adds PNG signature carving as a second recovery technique independent of FAT12 directory metadata. The carver scans the complete read-only image for the eight-byte PNG signature and parses bounded PNG chunks until a valid IEND chunk is found.

A carved PNG result carries a source byte range, a generated evidence name, the `signature_carving_png` recovery method, and the `content_validated` state when the PNG structure is internally consistent. The result does not claim an original filename, original directory, deletion time, or original allocation chain. It is presented as a discovered file structure rather than a metadata-proven file record.

The initial implementation validates PNG chunk boundaries, the IHDR placement and size, and the terminal IEND chunk. It does not yet validate PNG CRC values, decode pixels, reconstruct fragmented PNG data, recover all image formats, or infer the file’s original location. A signature match without a complete valid chunk sequence must not be exported as a validated PNG candidate.

The multi-method fixture contains one deleted FAT12 text file recovered through preserved directory metadata and one valid PNG embedded outside the FAT12 allocation structures. This demonstrates that metadata recovery and signature carving are distinct methods with distinct provenance and confidence labels.
