//! Distortion metrics: SAD, SATD (4x4 Hadamard), SSD.

#[inline]
pub fn sad(a: &[u8], sa: usize, b: &[u8], sb: usize, w: usize, h: usize) -> u32 {
    let mut s = 0u32;
    for r in 0..h {
        let ra = &a[r * sa..r * sa + w];
        let rb = &b[r * sb..r * sb + w];
        s += ra.iter().zip(rb).map(|(&x, &y)| x.abs_diff(y) as u32).sum::<u32>();
    }
    s
}

/// SAD of a 16-wide block (fast path).
#[inline]
pub fn sad16(a: &[u8], sa: usize, b: &[u8], sb: usize, h: usize) -> u32 {
    let mut s = 0u32;
    for r in 0..h {
        let (Some(ra), Some(rb)) = (a.get(r * sa..).and_then(|x| x.first_chunk::<16>()), b.get(r * sb..).and_then(|x| x.first_chunk::<16>())) else {
            break;
        };
        let mut acc = 0u16;
        for i in 0..16 {
            acc += ra[i].abs_diff(rb[i]) as u16;
        }
        s += acc as u32;
    }
    s
}

#[cfg(test)]
fn hadamard4x4_sum(d: &[i32; 16]) -> u32 {
    let mut t = [0i32; 16];
    for i in 0..4 {
        let a0 = d[i * 4] + d[i * 4 + 1];
        let a1 = d[i * 4] - d[i * 4 + 1];
        let a2 = d[i * 4 + 2] + d[i * 4 + 3];
        let a3 = d[i * 4 + 2] - d[i * 4 + 3];
        t[i * 4] = a0 + a2;
        t[i * 4 + 1] = a1 + a3;
        t[i * 4 + 2] = a0 - a2;
        t[i * 4 + 3] = a1 - a3;
    }
    let mut s = 0u32;
    for i in 0..4 {
        let a0 = t[i] + t[4 + i];
        let a1 = t[i] - t[4 + i];
        let a2 = t[8 + i] + t[12 + i];
        let a3 = t[8 + i] - t[12 + i];
        s += (a0 + a2).unsigned_abs() + (a1 + a3).unsigned_abs() + (a0 - a2).unsigned_abs() + (a1 - a3).unsigned_abs();
    }
    s
}

/// SATD of a strip of `N` columns (N/4 side-by-side 4x4 blocks) and 4 rows, computed lane-wise so the compiler
/// can vectorise it. Uses |p+q| + |p-q| = 2 max(|p|, |q|) for the last butterfly, which also absorbs the /2.
#[inline(always)]
fn satd_strip<const N: usize, const H: usize>(a: &[u8], sa: usize, b: &[u8], sb: usize) -> u32 {
    let mut d = [[0i16; N]; 4];
    for r in 0..4 {
        let ra = &a[r * sa..r * sa + N];
        let rb = &b[r * sb..r * sb + N];
        for i in 0..N {
            d[r][i] = ra[i] as i16 - rb[i] as i16;
        }
    }
    let mut v = [[0i16; N]; 4];
    for i in 0..N {
        let s01 = d[0][i] + d[1][i];
        let d01 = d[0][i] - d[1][i];
        let s23 = d[2][i] + d[3][i];
        let d23 = d[2][i] - d[3][i];
        v[0][i] = s01 + s23;
        v[1][i] = s01 - s23;
        v[2][i] = d01 + d23;
        v[3][i] = d01 - d23;
    }
    let mut total = 0u32;
    for row in &v {
        let mut hs = [0i16; H];
        let mut hd = [0i16; H];
        for k in 0..H {
            hs[k] = row[2 * k] + row[2 * k + 1];
            hd[k] = row[2 * k] - row[2 * k + 1];
        }
        for k in 0..H / 2 {
            total += hs[2 * k].unsigned_abs().max(hs[2 * k + 1].unsigned_abs()) as u32;
            total += hd[2 * k].unsigned_abs().max(hd[2 * k + 1].unsigned_abs()) as u32;
        }
    }
    total
}

/// SATD of a strip of `B` side-by-side 4x4 blocks (`N` = 4 * `B` columns, 4 rows) with the columns split out
/// first: lane `r * B + blk` of `x[j]` is column `j` of row `r` of block `blk`, so both butterfly passes are
/// lane-wise adds between whole arrays (rows `r` and `r + 1` are `B` lanes apart) instead of shuffles inside a
/// row. Faster than [`satd_strip`] for the narrow strips (4 and 8 columns); same result.
#[inline(always)]
fn satd_strip_cols<const N: usize, const B: usize>(a: &[u8], sa: usize, b: &[u8], sb: usize) -> u32 {
    let mut x = [[0i16; N]; 4];
    for r in 0..4 {
        let (Some(ra), Some(rb)) = (a.get(r * sa..).and_then(|v| v.first_chunk::<N>()), b.get(r * sb..).and_then(|v| v.first_chunk::<N>())) else {
            return 0;
        };
        for j in 0..4 {
            for blk in 0..B {
                x[j][r * B + blk] = ra[4 * blk + j] as i16 - rb[4 * blk + j] as i16;
            }
        }
    }
    let h0: [i16; N] = std::array::from_fn(|k| x[0][k] + x[1][k]);
    let h1: [i16; N] = std::array::from_fn(|k| x[0][k] - x[1][k]);
    let h2: [i16; N] = std::array::from_fn(|k| x[2][k] + x[3][k]);
    let h3: [i16; N] = std::array::from_fn(|k| x[2][k] - x[3][k]);
    let g: [[i16; N]; 4] = [
        std::array::from_fn(|k| h0[k] + h2[k]),
        std::array::from_fn(|k| h1[k] + h3[k]),
        std::array::from_fn(|k| h0[k] - h2[k]),
        std::array::from_fn(|k| h1[k] - h3[k]),
    ];
    // every term is at most 8 * 255 = 2040 and a lane gets 4 of them: i16 can't overflow
    let mut acc = [0i16; N];
    for g in &g {
        // rows 0 and 1 / 2 and 3: sums in lanes 0..2B, differences in 2B..4B
        let e: [i16; N] = std::array::from_fn(|k| {
            let (half, i) = (k / (2 * B), k % (2 * B));
            let (r, blk) = (2 * (i / B), i % B);
            let (p, q) = (g[r * B + blk], g[(r + 1) * B + blk]);
            if half == 0 { p + q } else { p - q }
        });
        // last butterfly (rows 01 against 23): |p+q| + |p-q| = 2 max(|p|, |q|), which also absorbs the /2
        for half in 0..2 {
            for i in 0..B {
                let (p, q) = (e[half * 2 * B + i], e[half * 2 * B + B + i]);
                acc[half * B + i] += p.max(-p).max(q.max(-q));
            }
        }
    }
    acc.iter().map(|&v| v as u32).sum()
}

/// Sum of absolute 4x4 Hadamard-transformed differences (halved), for w, h multiples of 4.
pub fn satd(a: &[u8], sa: usize, b: &[u8], sb: usize, w: usize, h: usize) -> u32 {
    let mut total = 0;
    for by in (0..h).step_by(4) {
        let (ao, bo) = (by * sa, by * sb);
        match w {
            16 => total += satd_strip::<16, 8>(&a[ao..], sa, &b[bo..], sb),
            8 => total += satd_strip_cols::<8, 2>(&a[ao..], sa, &b[bo..], sb),
            4 => total += satd_strip_cols::<4, 1>(&a[ao..], sa, &b[bo..], sb),
            _ => {
                for bx in (0..w).step_by(4) {
                    total += satd_strip_cols::<4, 1>(&a[ao + bx..], sa, &b[bo + bx..], sb);
                }
            }
        }
    }
    total
}

pub fn ssd(a: &[u8], sa: usize, b: &[u8], sb: usize, w: usize, h: usize) -> u64 {
    let mut s = 0u64;
    for r in 0..h {
        let ra = &a[r * sa..r * sa + w];
        let rb = &b[r * sb..r * sb + w];
        s += ra.iter().zip(rb).map(|(&x, &y)| (x as i32 - y as i32).pow(2) as u32).sum::<u32>() as u64;
    }
    s
}

/// Variance*N (sum of squares minus square of sum / N) of a block.
pub fn block_var(a: &[u8], sa: usize, w: usize, h: usize) -> u32 {
    let mut s = 0u32;
    let mut ss = 0u32;
    for r in 0..h {
        for &v in &a[r * sa..r * sa + w] {
            s += v as u32;
            ss += (v as u32) * (v as u32);
        }
    }
    ss - ((s as u64 * s as u64) / (w * h) as u64) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn satd_of_dc_difference() {
        let a = [10u8; 16];
        let b = [0u8; 16];
        // DC-only difference of 10: hadamard DC = 160, halved = 80.
        assert_eq!(satd(&a, 4, &b, 4, 4, 4), 80);
        assert_eq!(sad(&a, 4, &b, 4, 4, 4), 160);
    }

    #[test]
    fn satd_matches_reference() {
        let mut seed = 7u32;
        let mut rnd = || {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            (seed >> 16) as u8
        };
        let a: Vec<u8> = (0..16 * 16).map(|_| rnd()).collect();
        let b: Vec<u8> = (0..16 * 16).map(|_| rnd()).collect();
        for (w, h) in [(16, 16), (8, 8), (4, 4), (16, 8), (8, 16), (12, 4), (8, 4), (4, 8)] {
            let mut want = 0;
            for by in (0..h).step_by(4) {
                for bx in (0..w).step_by(4) {
                    let mut d = [0i32; 16];
                    for r in 0..4 {
                        for c in 0..4 {
                            d[r * 4 + c] = a[(by + r) * 16 + bx + c] as i32 - b[(by + r) * 16 + bx + c] as i32;
                        }
                    }
                    want += hadamard4x4_sum(&d);
                }
            }
            assert_eq!(satd(&a, 16, &b, 16, w, h), want / 2, "{w}x{h}");
        }
    }

    #[test]
    fn satd_extremes_match_reference() {
        // largest possible differences (0 against 255 in every sign pattern) must not overflow the i16 lanes
        let patterns: [fn(usize, usize) -> bool; 4] = [|_, _| true, |x, y| (x + y) % 2 == 0, |x, _| x % 2 == 0, |x, y| (x / 2 + y) % 3 == 0];
        for pat in patterns {
            let a: Vec<u8> = (0..16 * 16).map(|i| if pat(i % 16, i / 16) { 255 } else { 0 }).collect();
            let b: Vec<u8> = (0..16 * 16).map(|i| if pat(i % 16, i / 16) { 0 } else { 255 }).collect();
            for (w, h) in [(16, 16), (8, 8), (4, 4), (8, 4), (4, 8), (12, 4)] {
                let mut want = 0;
                for by in (0..h).step_by(4) {
                    for bx in (0..w).step_by(4) {
                        let mut d = [0i32; 16];
                        for r in 0..4 {
                            for c in 0..4 {
                                d[r * 4 + c] = a[(by + r) * 16 + bx + c] as i32 - b[(by + r) * 16 + bx + c] as i32;
                            }
                        }
                        want += hadamard4x4_sum(&d);
                    }
                }
                assert_eq!(satd(&a, 16, &b, 16, w, h), want / 2, "{w}x{h}");
            }
        }
    }
}
