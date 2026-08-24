# FAT12 v1 Recovery Boundary

DiskTrace v0.1 implements a deliberately narrow FAT12 recovery path for disk-image files. The supported case is a deleted short-name file in the FAT12 root directory where the directory entry remains available and the file’s cluster chain remains readable.

The parser supports 512-byte-sector FAT12 images with a readable boot sector, one or more FAT copies, a fixed root directory, short 8.3 directory entries, and cluster-chain extraction. It reports deleted root-directory entries only. It does not claim recovery of long filenames, subdirectory entries, fragmented files whose chains have been cleared or overwritten, damaged FAT structures, deleted partition reconstruction, FAT16/FAT32, exFAT, NTFS, APFS, or physical media errors.

A deleted FAT directory entry does not retain the first character of its original short filename. DiskTrace represents that unavailable character as `?` and never presents the original name as proven. The initial fixture contains a controlled deleted file whose preserved short-name bytes render as `?ELETED.TXT`; the exported filename is sanitized independently of the evidence display name.

The first fixture models a readable deleted file with a retained FAT12 chain. This is a controlled positive case, not a claim that every deleted FAT12 file remains recoverable. The fixture also contains a normal active entry to prove that the scanner does not treat all root-directory entries as deleted candidates.
