use ndarray::Array2;

/// Bit-packed 2D boolean mask. Layout: row-major, words of u64.
/// Indexing convention matches `Array2<bool>`: `mask.get(y, x)`.
///
/// Replaces `Array2<bool>` (1 byte per cell) with 1 bit per cell, shrinking
/// the working set 8x and dramatically improving cache locality on the hot
/// path (every layout strategy probes the mask repeatedly per attempt).
#[derive(Debug, Clone)]
pub struct BitMask {
	bits: Vec<u64>,
	width: usize,
	height: usize,
	words_per_row: usize,
}

impl BitMask {
	pub fn from_fn<F: FnMut(usize, usize) -> bool>(height: usize, width: usize, mut f: F) -> Self {
		let words_per_row = width.div_ceil(64).max(1);
		let mut bits = vec![0u64; words_per_row * height.max(1)];
		for y in 0..height {
			for x in 0..width {
				if f(y, x) {
					let idx = y * words_per_row + x / 64;
					bits[idx] |= 1u64 << (x % 64);
				}
			}
		}
		Self {
			bits,
			width,
			height,
			words_per_row,
		}
	}

	pub fn zeros(height: usize, width: usize) -> Self {
		Self::from_fn(height, width, |_, _| false)
	}

	pub fn from_array(arr: &Array2<bool>) -> Self {
		let (h, w) = arr.dim();
		Self::from_fn(h, w, |y, x| arr[[y, x]])
	}

	#[inline]
	pub fn get(&self, y: usize, x: usize) -> bool {
		if y >= self.height || x >= self.width {
			return false;
		}
		let idx = y * self.words_per_row + x / 64;
		(self.bits[idx] >> (x % 64)) & 1 == 1
	}

	#[inline]
	pub fn set(&mut self, y: usize, x: usize, value: bool) {
		if y >= self.height || x >= self.width {
			return;
		}
		let idx = y * self.words_per_row + x / 64;
		let bit = 1u64 << (x % 64);
		if value {
			self.bits[idx] |= bit;
		} else {
			self.bits[idx] &= !bit;
		}
	}

	#[inline]
	pub fn nrows(&self) -> usize {
		self.height
	}

	#[inline]
	pub fn ncols(&self) -> usize {
		self.width
	}

	/// Iterate all set cells as (y, x). Lazy - no intermediate allocation.
	pub fn indexed_set(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
		let words_per_row = self.words_per_row;
		let width = self.width;
		(0..self.height).flat_map(move |y| {
			(0..words_per_row).flat_map(move |w| {
				let mut word = self.bits[y * words_per_row + w];
				let base_x = w * 64;
				std::iter::from_fn(move || {
					while word != 0 {
						let bit = word.trailing_zeros() as usize;
						word &= word - 1;
						let x = base_x + bit;
						if x < width {
							return Some((y, x));
						}
					}
					None
				})
			})
		})
	}

	/// Count set bits.
	pub fn count_ones(&self) -> usize {
		self.bits.iter().map(|w| w.count_ones() as usize).sum()
	}

	/// Clear all bits in rect [(y..y+h), (x..x+w)); returns the count of
	/// bits actually flipped from set to unset.
	///
	/// Word-level fast path: full 64-bit-aligned middle words are cleared in
	/// one mask AND. Partial words at the rect's left/right edges fall back
	/// to a per-bit shifted mask. This is the placement hot path.
	pub fn clear_rect(&mut self, x: usize, y: usize, w: usize, h: usize) -> usize {
		if w == 0 || h == 0 {
			return 0;
		}
		let x_end = (x + w).min(self.width);
		let y_end = (y + h).min(self.height);
		if x >= x_end || y >= y_end {
			return 0;
		}

		let mut cleared = 0usize;
		for py in y..y_end {
			let row_start = py * self.words_per_row;
			let mut px = x;
			while px < x_end {
				let word_idx = row_start + px / 64;
				let bit_in_word = px % 64;
				// Bits remaining in this word and in this rect-row.
				let bits_in_word = 64 - bit_in_word;
				let bits_in_rect = x_end - px;
				let span = bits_in_word.min(bits_in_rect);

				let mask = if span == 64 {
					u64::MAX
				} else {
					((1u64 << span) - 1) << bit_in_word
				};

				let word = &mut self.bits[word_idx];
				let to_clear = *word & mask;
				cleared += to_clear.count_ones() as usize;
				*word &= !mask;

				px += span;
			}
		}
		cleared
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_fn_and_get_round_trip() {
		let m = BitMask::from_fn(5, 7, |y, x| (y + x) % 2 == 0);
		for y in 0..5 {
			for x in 0..7 {
				assert_eq!(m.get(y, x), (y + x) % 2 == 0);
			}
		}
		assert_eq!(m.nrows(), 5);
		assert_eq!(m.ncols(), 7);
	}

	#[test]
	fn set_and_clear() {
		let mut m = BitMask::zeros(4, 100);
		assert!(!m.get(2, 65));
		m.set(2, 65, true);
		assert!(m.get(2, 65));
		assert_eq!(m.count_ones(), 1);
		m.set(2, 65, false);
		assert!(!m.get(2, 65));
		assert_eq!(m.count_ones(), 0);
	}

	#[test]
	fn count_ones_matches_iter() {
		let m = BitMask::from_fn(11, 130, |y, x| (y * 13 + x * 7) % 5 == 0);
		let count: usize = m.indexed_set().count();
		assert_eq!(m.count_ones(), count);
	}

	#[test]
	fn clear_rect_word_aligned_and_unaligned() {
		// Set everything, then clear a rect crossing word boundaries.
		let mut m = BitMask::from_fn(6, 200, |_, _| true);
		let initial = m.count_ones();
		assert_eq!(initial, 6 * 200);

		// Rect: y=2, x=63 (just before word boundary), w=70, h=3
		let cleared = m.clear_rect(63, 2, 70, 3);
		assert_eq!(cleared, 70 * 3);
		assert_eq!(m.count_ones(), initial - 70 * 3);

		// Verify cells in the rect are clear, edges are still set.
		for y in 2..5 {
			for x in 63..133 {
				assert!(!m.get(y, x), "cell ({y},{x}) should be cleared");
			}
		}
		assert!(m.get(2, 62));
		assert!(m.get(2, 133));
		assert!(m.get(1, 100));
		assert!(m.get(5, 100));
	}

	#[test]
	fn clear_rect_clamps_to_bounds() {
		let mut m = BitMask::from_fn(4, 10, |_, _| true);
		let cleared = m.clear_rect(8, 2, 100, 100);
		assert_eq!(cleared, 2 * 2);
		assert_eq!(m.count_ones(), 4 * 10 - 4);
	}

	#[test]
	fn clear_rect_returns_only_set_bits_flipped() {
		let mut m = BitMask::from_fn(4, 130, |y, x| x == y);
		// rect covering rows 0..4, cols 0..130 should clear exactly 4 cells (the diagonal).
		let cleared = m.clear_rect(0, 0, 130, 4);
		assert_eq!(cleared, 4);
		assert_eq!(m.count_ones(), 0);
	}

	#[test]
	fn indexed_set_returns_all_set_cells() {
		let mut m = BitMask::zeros(3, 130);
		m.set(0, 0, true);
		m.set(1, 64, true);
		m.set(2, 129, true);
		let mut got: Vec<_> = m.indexed_set().collect();
		got.sort();
		assert_eq!(got, vec![(0, 0), (1, 64), (2, 129)]);
	}
}
