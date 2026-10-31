# Evidence: MS-ABS – address book files (the client's offline address book)

Source: [MS-ABS] "Address Book File Structure", edition 20250520, 79 pp. Product list p. 74 up to SfB LTSC 2024. Download URL from in-band provisioning `absInternalServerUrl`/`absExternalServerUrl` (MS-SIPREGE, file 04, ServerConfiguration).

| ID | Requirement | Evidence |
|---|---|---|
| AB-01 | Fetch via **HTTP GET** of `<base URL>/<file name>`; file names `F-XXXX.lsabs` (full), `D-XXXX-YYYY.lsabs` (delta), `C-XXXX-YYYY.lsabs` (compact delta), `.dabs` (device version, without groups); XXXX/YYYY = 4-digit hex, days since 2001-01-01 UTC | §1.3, pp. 9–10 |
| AB-02 | Purpose: local search for users/contacts/groups, reverse lookup of incoming numbers; number normalisation also for own input | §1.3–1.3.1, pp. 10–11 |
| AB-03 | File = sequence of COMPRESSED_BLOCK (12-byte LE header: CRC32 of the uncompressed data, CompressedSize ≤ 0x10000, DecompressedSize; equal ⇒ uncompressed) | §1.3.2, p. 11; §2.2.1–2.2.2, pp. 17–18 |
| AB-04 | Compression: LZ77 variant; token groups with a 32-bit type word (LE), literal or run (offset 13 bits −1, length 3 bits −3, extension via a shared nibble (−10), a byte (−25) and 0xFF + 16 bits); CRC32 = standard CRC (table printed) | §5.1–5.5.2, pp. 63–69 |
| AB-05 | Unpacked (little endian): full = ABF_FULL_HEADER (FileType GUID `76 6c e1 44 fd 0a a9 40 8b 63 5f e9 b0 81 73 8f`, CreationDate, NumberOfAttributes, UseNormalizationRules, MaximumAttributeId, 128-byte SourceStream) → ABF_NORMALIZATION_RULES → ABF_ATTRIBUTES → ABF_CONTACTS (sentinel with a null GUID) → ABF_FULL_TRAILER (Hash16, NumberOfContacts) → ABF_TRAILER_LENGTH; delta GUID `16 c1 4b b5 08 90 c7 47 b9 bd f3 bb 1a 0a b6 eb`, compact delta `34 17 7d f7 87 ae 2b 4d 09 a0 8e e9 ba 89 4a 04` with deleted contacts or only changed attributes | §1.3.3–1.3.4, pp. 11–13; §2.2.3–2.2.16, pp. 18–25 |
| AB-06 | ABF_ATTRIBUTE: flags (type string/binary/proxy address, EMAIL, REQUIRED, normalisation control, EXCLUDE/INCLUDE, GROUPTYPE, DEVICE, client field in the highest byte: 0 proxyAddresses … 0x08 msRTCSIP-PrimaryUserAddress, 0x09 telephoneNumber, 0x0B mobile, 0x0E mail, 0x0F groupType, 0x10 manager, 0xFF ignore), name UTF-8 NUL-terminated | §2.1.1–2.1.4, pp. 14–17; §2.2.7, p. 21 |
| AB-07 | ABF_CONTACT: objectGUID (16 bytes), NumberOfAttributes, attributes (ID 1 byte if MaximumAttributeId < 256, otherwise 2; binary attributes with a 2-byte length; strings NUL-terminated UTF-8) | §2.2.10–2.2.11, pp. 22–23 |
| AB-08 | Normalisation rules: pairs "regex line CRLF replacement line CRLF", the first matching rule applies; special rule `E164` (≤15 digits, blanks/separators ignored ⇒ `tel:+<digits>`); regex syntax **.NET** (MC-RegEx; products from OCS 2007 R2: .NET 3.5) | §2.2.5, p. 20; <8> p. 75 |
| AB-09 | Read the trailer at the end of the file: last 4 bytes = LengthOfTrailer | §2.2.14, p. 24 |

### Checked
- Date coding: 0x0A8C = 2008-05-24 (Saturday) ✓, 0x0A10 = 2008-01-21 (Monday) ✓, 0x0A11 = 2008-01-22 ✓ (pp. 10, 26).
- CRC table (pp. 63–64): all 256 entries = standard CRC-32 (0xEDB88320) ✓.
- Own encoder after the pseudo code §5.4 + decoder after §5.5.2: round trip over 40 test inputs without errors.
- Hex example pp. 30–31: objectGUID `8c36ad0a-5e97-46dd-8d5b-255140c52b00` appears as `0a ad 36 8c 97 5e dd 46 …` (Windows GUID byte order) ✓.

## Conflicts
- K-AB-1 (documented): §5.2 (p. 64) says "bit 0 = first token"; the decoder pseudo code §5.5.2 (p. 67) starts with mask 0x80000000 (**bit 31 = first token**). The round-trip test is only consistent with MSB first; the pseudo code is authoritative. Confirmed by the compressed example §3.2 (pp. 59–60), see later findings.
- K-AB-2 (documented): The bit diagrams of ABF_ATTRIBUTE (p. 21) and ABF_CONTACT (p. 22) show Id before Length; the field description and the hex example (p. 30: `0e 00 | 14 00 | 00 00 00 10 | "manager\0"`, length 14 only fits this way) show **Length before Id**. Implementation: Length first.
- K-AB-3: The hash function is implementation-specific (.NET GetHashCode, <13> p. 75) ⇒ the client cannot recompute the hash, only compare it (BaseFileHash of a delta = hash of the existing full file).
- K-AB-4 (documented, found 2026-10-27): The examples §3.1 (decompressed, pp. 28–31) and §3.2/§3.3 (compressed, pp. 59–61) do not match exactly. Decompressing §3.2 gives the §3.1 bytes except 8 bytes: one run copies `e.com` from `proseware.com`, so the compressed file contains `urtese.com` where §3.1 shows `urtest.com` (offsets 0xABB, 0xB6F, 0xB7D, 0xB9D, 0xC46, 0xC54, 0xCEE, 0xCFC). The CRC 0xA25C656E printed in §3.3 matches neither version (CRC-32 of §3.1 = 0x3C94D862, of the decompressed §3.2 = 0x1FECC257). The examples were evidently edited after they were generated; the CRC algorithm itself is the standard CRC-32 (the printed table in §5.1 equals it entry by entry). Consequence: the example cannot serve as a CRC test vector.

## Later findings (2026-10-27)
- K-AB-1 **confirmed by the specification's own example**: the compressed block of §3.2 (CompressedSize 0x483, DecompressedSize 0xD72) decompresses with bit 31 as the first token and consumes exactly 0x483 bytes; with bit 0 first it runs past the end of the input. Recomputed in `crates/abs` (test `example_block_decompresses_msb_first`).
- File-name dates recomputed in `crates/abs`: 0x0A8C = 2008-05-24 (Saturday, §1.3 p. 10), 0x0A10 = 2008-01-21 (Monday) and 0x0A11 = 2008-01-22 (Tuesday) (§3.1 p. 26).
