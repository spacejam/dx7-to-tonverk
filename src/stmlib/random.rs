// Copyright 2025 Tyler Neely (tylerneely@gmail.com).
// Copyright 2021 Emilie Gillet (emilie.o.gillet@gmail.com)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
//
// See http://creativecommons.org/licenses/MIT/ for more information.

//! Fast 16-bit pseudo random number generator

use std::cell::Cell;

thread_local! {
    static RNG_STATE: Cell<u32> = Cell::new(0x21);
}

/// Fast pseudo random number generator (Linear Congruential Generator)
pub struct Random;

impl Random {
    /// Generates a 32-bit random word
    #[inline]
    pub fn get_word() -> u32 {
        RNG_STATE.with(|state| {
            let new_state = state.get().wrapping_mul(1664525).wrapping_add(1013904223);
            state.set(new_state);
            new_state
        })
    }

    /// Generates a random float in the range [0.0, 1.0)
    #[inline]
    pub fn get_float() -> f32 {
        Self::get_word() as f32 / 4294967296.0
    }
}
