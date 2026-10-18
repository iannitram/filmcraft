*Machine-drafted by Qwen 2026-10-16, reviewed and corrected by Claude 2026-10-17.*

# Goal 3 — 10-bit / 4:2:2 Camera Media Support

## Scope

Enable software decoding of H.264 High 10 / High 4:2:2 and HEVC Main 10 /
Main 4:2:2 10 streams (typical camera media) without hardware acceleration.
This is Goal #3 in ROADMAP.md (30–50 h, G3/M9 milestone).

## Architecture overview

H.264 and HEVC use different memory models:

- **H.264** (`crates/h264/`): `Planes` struct in `picture.rs` holds three
  `Vec<u8>` buffers (Y, Cb, Cr).  Transforms, deblocking (`deblock.rs` line 1:
  "4:2:0, 8-bit") and chroma prediction are written for 8-bit 4:2:0.
- **HEVC** (`crates/hevc/`): Uses a `Sample` trait (`u8`/`u16`,
  `picture.rs:22`).  10- and 12-bit already decode; the blocker is
  `Sps::check_supported()` rejecting every chroma format other than 4:2:0,
  plus 4:2:0 assumptions in the slice decoder.

## H.264 migration

### 1. `Planes` → widen sample buffers

**File:** `crates/h264/src/picture.rs`

`Planes` has `y: Vec<u8>, cb: Vec<u8>, cr: Vec<u8>`.  Widen to `Vec<u16>`
for 10/12-bit content, or introduce a `Sample` trait mirroring HEVC's
approach.  `MbState` (motion vectors, QP) remains unchanged.

**Change count:** ~1 file, ~120 lines.

### 2. `Sps::check_supported()` — lift bans

**File:** `crates/h264/src/params.rs:518`

Current guards: `self.chroma_array_type() != 1` (line 522, only 4:2:0) and
`bit_depth_luma != 8 || bit_depth_chroma != 8` (line 525).  Lift each one
only when the code it protects handles the new format; the decoder entry
point (`decoder.rs:338`) calls this function.  The SPS parser already reads
the High-profile fields (`chroma_format_idc` etc., `params.rs:350`).

**Change count:** ~5 lines removed.

### 3. Chroma geometry — `MbWidthC` / `MbHeightC` scaling

4:2:2 has `MbWidthC = 8, MbHeightC = 16`; 4:2:0 has `MbWidthC = 8, MbHeightC = 8`.
All chroma block indexing must use SPS-derived values.

| File | What changes |
|------|--------------|
| `slicedec.rs:1001` | `pred_chroma` call — chroma origin is `mb_y * 8`; use `MbHeightC` |
| `slicedec.rs:1030` | `write_pcm` chroma copy — 8×16 chroma per MB for 4:2:2 |
| `cavlc.rs` | Already has the 4:2:2 chroma DC tables (`total_zeros_2x4`, lines 62 and 154; `nC = -2`): wire them in |
| `cabac_mb.rs:35` | `CHROMA_DC_CTX` is the 4:2:0 ctxIdxInc (`Min(i / NumC8x8, 2)` with NumC8x8 = 1); 4:2:2 chroma DC has 8 coefficients and NumC8x8 = 2 |
| `deblock.rs` | Chroma edge filtering — 4:2:0-specific edge positions |
| `transform.rs:75` | `chroma_dc_dequant_420` (2×2) → add the 2×4 chroma DC transform and its QP' + 3 scaling (8.5.11) |
| `intra.rs:293` | `pred_chroma` — `W`/`H` are fixed at 8; 4:2:2 is 8×16 (and plane prediction uses different constants) |
| `inter.rs` | `mc_chroma` — for 4:2:2 the vertical chroma MV is in quarter-sample units (`yIntC = yAL + (mvCLX[1] >> 2)`, `yFracC = (mvCLX[1] & 3) << 1`, 8.4.2.2.2) and the chroma block is full height |

**Change count:** ~7 files, ~200 lines.

### 4. CABAC chroma DC residual

**File:** `crates/h264/src/cabac_mb.rs`

The CAVLC 2×4 chroma DC tables already exist.  CABAC residual decoding for
ctxBlockCat 3 (chroma DC) is called with 4 coefficients (`cabac_mb.rs:498`); 4:2:2 needs 8 and the
NumC8x8 = 2 ctxIdxInc for the significance map.

**Change count:** ~1 file, ~30 lines.

### 5. Deblocking filter

**File:** `crates/h264/src/deblock.rs`

Line 1 documents "4:2:0, 8-bit".  Chroma deblocking assumes the 4:2:0 block
layout (`filter_vertical` / `filter_horizontal` with `chroma = true`); 4:2:2
adds horizontal chroma edges.  Add `MbHeightC`-driven loop bounds.

**Change count:** ~1 file, ~50 lines.

### 6. Output frames

`crates/codecs/src/video.rs` (`H264Decoder`, around line 317) always emits
`PixelData::Yuv8` with `Chroma::C420`.  Emit `PixelData::Yuv16` with the
real bit depth and chroma format, as the HEVC and AV1 wrappers already do.

### 7. Tests

Generate H.264 High 10 / High 4:2:2 streams with ffmpeg into
`target/fixtures/` (never commit media) and compare decoded frames with
ffmpeg as the oracle, next to the existing `crates/h264/tests/conformance.rs`.
(`crates/codecs/tests/y4m_file.rs` already tests a 4:2:2 10-bit *Y4M* file,
which exercises the Y4M reader, not the H.264 decoder.)

**Change count:** ~1 file, ~60 lines.

---

## HEVC migration

### 1. Lift the `Sps::check_supported()` bans

**File:** `crates/hevc/src/params.rs:636`

```rust
if self.chroma_format_idc != 1 {   // rejects monochrome, 4:2:2, 4:4:4
    return unsupported(...);
}
...
if self.range_extension { return unsupported("SPS range extension tools"); }
if self.extension_flags & 0x7f != 0 { return unsupported(...); }
```

Remove the chroma guard once the slice decoder handles 4:2:2.  Camera
4:2:2 streams (Main 4:2:2 10 is a range-extensions profile) may also set
`range_extension`, so the SPS range-extension tools (and the matching PPS
check, `params.rs:823-828`) must be implemented and lifted too, or those
streams still fail.  The bit-depth guard already allows up to 12 bits.

**Change count:** a few lines removed, plus whatever range-extension tools
real files use.

### 2. Block geometry

`Sps::sub_width_c()` / `sub_height_c()` (`params.rs:605-610`) already
return the right values for 4:2:2 (2, 1), but the slice decoder does not use
them: `slicedec.rs:972` hardcodes `x0 / 2, y0 / 2, log2 - 1`.  For 4:2:2 each
chroma transform block is half width and full height, coded as two
vertically stacked square blocks, each with its own `cbf_cb`/`cbf_cr`
(`transform_tree`, `slicedec.rs:894-906`).

**Change count:** ~3 files, ~80 lines.

### 3. SAO / deblocking — chroma

SAO and deblocking live in `crates/hevc/src/filter.rs`.  Chroma SAO and
chroma deblocking edges must use `sub_width_c`/`sub_height_c`; check every
place that halves a coordinate.

**Change count:** ~1 file, ~30 lines.

### 4. Intra prediction — chroma mode

**File:** `crates/hevc/src/slicedec.rs:881-890`

The chroma mode is derived for 4:2:0 only.  For 4:2:2 the derived mode goes
through the 4:2:2 mode conversion table (Table 8-3) before prediction.

**Change count:** ~1 file, ~20 lines.

### 5. Output frames

`crates/codecs/src/video.rs` (`HevcDecoder::convert`, lines 421-428) tags
every picture `Chroma::C420`; pass the real chroma format.

### 6. Tests

Add HEVC Main 4:2:2 10 ffmpeg-oracle tests next to
`crates/hevc/tests/conformance.rs`, with fixtures generated into
`target/fixtures/`.

**Change count:** ~1 file, ~40 lines.

---

## Shared infrastructure

### Codec pipeline frame buffers

The pipeline already carries 16-bit frames (`filmcraft_frame::PixelData::Yuv16`
with `chroma` and `bits`, used by the HEVC and AV1 wrappers in
`crates/codecs/src/video.rs`), so no new frame type is needed.  The HEVC
`Sample` trait (`crates/hevc/src/picture.rs:22`) can be extracted to a shared
module if H.264 adopts the same approach.

### Test fixtures

Generate 4:2:2 10-bit test streams with `ffmpeg` as an external oracle
(AGENTS.md §2) into `target/fixtures/` at test time.  Never commit media.

---

## Effort estimate

| Area | Hours |
|------|-------|
| H.264 `Planes` refactor | 6 |
| H.264 `check_supported` lift | 1 |
| H.264 chroma geometry | 8 |
| H.264 CABAC chroma DC | 3 |
| H.264 deblock/intra/inter | 6 |
| H.264 output + tests | 2 |
| HEVC `check_supported` lift + range extensions | 1+ |
| HEVC block geometry | 4 |
| HEVC SAO/deblock | 4 |
| HEVC intra chroma mode | 2 |
| HEVC output + tests | 2 |
| Shared `Sample` trait | 3 |
| Test fixtures | 2 |
| **Total** | **~42 h** (more if range-extension tools are needed) |

---

## Rejection points summary

| Decoder | File:Line | Current guard | Action |
|---------|-----------|---------------|--------|
| H.264 | `params.rs:522` | `chroma_array_type() != 1` | Lift |
| H.264 | `params.rs:525` | bit depth `!= 8` | Lift |
| HEVC | `params.rs:637` | `chroma_format_idc != 1` | Lift |
| HEVC | `params.rs:643, 646` | `range_extension`, `extension_flags & 0x7f` | Implement and lift range extensions |
| HEVC | `params.rs:824` | PPS `range_extension` | Same |
| H.264 | `picture.rs` | `Vec<u8>` hardcoded | Widen to `u16` |
| H.264 | `transform.rs:75` | `chroma_dc_dequant_420` | Add 2×4 |
| H.264 | `deblock.rs:1` | "4:2:0" in docstring & logic | Parameterize |
| H.264 | `cabac_mb.rs:35` | 4:2:0 chroma DC contexts | Add 4:2:2 |
| H.264 | `intra.rs:293` | 8×8 chroma pred | Block-size-aware |

---

## Dependencies

- No new dependencies; all changes within `crates/h264`, `crates/hevc`
  and `crates/codecs`.
- Hardware decode paths (`crates/platform/`) are unaffected.

---

## Acceptance criteria

1. Oracle tests pass for H.264 High 10 and High 4:2:2 streams.
2. Oracle tests pass for HEVC Main 10 and Main 4:2:2 10 streams.
3. `cargo xtask ci` (layer check, clippy, tests) passes.
4. No new `unwrap()`/`expect()`/`panic!` in affected crates; mutation-fuzz
   tests cover the new 4:2:2 paths.
