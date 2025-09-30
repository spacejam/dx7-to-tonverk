// Copyright 2021 Emilie Gillet.
// Rust port by Tyler Neely.
//
// Author: Emilie Gillet (emilie.o.gillet@gmail.com)
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

//! FM algorithms and routing structures

use super::operator::{render_operators, RenderFn};

use crate::{NUM_ALGORITHMS, NUM_OPERATORS};

// Opcode flag constants
const DESTINATION_MASK: u8 = 0x03;
const SOURCE_MASK: u8 = 0x30;
const SOURCE_FEEDBACK: u8 = 0x30;
const ADDITIVE_FLAG: u8 = 0x04;
const FEEDBACK_SOURCE_FLAG: u8 = 0x40;

// Helper macros for opcode construction (translated from C++ macros)
const fn mod_flags(n: u8) -> u8 {
    n << 4
}

const fn add_flags(n: u8) -> u8 {
    n | ADDITIVE_FLAG
}

const fn out_flags(n: u8) -> u8 {
    n
}

const FB_SRC: u8 = FEEDBACK_SOURCE_FLAG;
const FB_DST: u8 = mod_flags(3);
const FB: u8 = FB_SRC | FB_DST;
const NO_MOD: u8 = mod_flags(0);
const OUTPUT: u8 = add_flags(0);

/// Store information about all FM algorithms, and which functions to call
/// to render them.
pub struct Algorithms {
    render_calls: [[RenderCall; NUM_OPERATORS]; NUM_ALGORITHMS],
}

impl Algorithms {
    /// Creates and initializes a new algorithm manager
    pub fn new() -> Self {
        let mut algorithms = Self {
            render_calls: [[RenderCall::default(); NUM_OPERATORS]; NUM_ALGORITHMS],
        };
        algorithms.init();
        algorithms
    }

    /// Initializes all algorithms by compiling their opcodes
    pub fn init(&mut self) {
        for i in 0..NUM_ALGORITHMS {
            self.compile(i);
        }
    }

    /// Returns the render call for a specific algorithm and operator
    #[inline]
    pub fn render_call(&self, algorithm: usize, op: usize) -> &RenderCall {
        &self.render_calls[algorithm][op]
    }

    /// Checks if an operator is a modulator (not a carrier)
    #[inline]
    pub fn is_modulator(&self, algorithm: usize, op: usize) -> bool {
        (Self::opcodes()[algorithm][op] & DESTINATION_MASK) != 0
    }

    fn compile(&mut self, algorithm: usize) {
        let opcodes = Self::opcodes()[algorithm];
        let mut i = 0;

        while i < NUM_OPERATORS {
            let opcode = opcodes[i];
            let mut n = 1;

            // Try to chain operators together
            while i + n < NUM_OPERATORS {
                let from = opcodes[i + n - 1];
                let to = (opcodes[i + n] & SOURCE_MASK) >> 4;

                let has_additive = (from & ADDITIVE_FLAG) != 0;
                let broken = (from & DESTINATION_MASK) != to;

                if has_additive || broken {
                    if to == (opcode & DESTINATION_MASK) {
                        n = 1;
                    }
                    break;
                }
                n += 1;
            }

            // Try to find if a pre-compiled renderer is available for this chain
            for _attempt in 0..2 {
                let out_opcode = opcodes[i + n - 1];
                let additive = (out_opcode & ADDITIVE_FLAG) != 0;

                let mut modulation_source = -3;
                if (opcode & SOURCE_MASK) == 0 {
                    modulation_source = -1;
                } else if (opcode & SOURCE_MASK) != SOURCE_FEEDBACK {
                    modulation_source = -2;
                } else {
                    for j in 0..n {
                        if (opcodes[i + j] & FEEDBACK_SOURCE_FLAG) != 0 {
                            modulation_source = j as i32;
                        }
                    }
                }

                if let Some(render_fn) = Self::get_renderer(n, modulation_source, additive) {
                    self.render_calls[algorithm][i] = RenderCall {
                        render_fn,
                        n,
                        input_index: ((opcode & SOURCE_MASK) >> 4) as usize,
                        output_index: (out_opcode & DESTINATION_MASK) as usize,
                    };
                    break;
                } else if n > 1 {
                    n = 1;
                }
            }
            i += n;
        }
    }

    fn get_renderer(n: usize, modulation_source: i32, additive: bool) -> Option<RenderFn> {
        for specs in Self::renderers() {
            if specs.n == 0 {
                break;
            }
            if specs.n == n
                && specs.modulation_source == modulation_source
                && specs.additive == additive
            {
                return Some(specs.render_fn);
            }
        }
        None
    }

    fn opcodes() -> &'static [[u8; NUM_OPERATORS]; NUM_ALGORITHMS] {
        match NUM_OPERATORS {
            6 => unsafe { &*(OPCODES_6.as_ptr() as *const [[u8; NUM_OPERATORS]; NUM_ALGORITHMS]) },
            _ => panic!("Unsupported number of operators"),
        }
    }

    fn renderers() -> &'static [RendererSpecs] {
        match NUM_OPERATORS {
            6 => &RENDERERS_6,
            _ => &[],
        }
    }
}

impl Default for Algorithms {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a render call for an operator or chain of operators
#[derive(Debug, Clone, Copy)]
pub struct RenderCall {
    /// Function pointer to the renderer
    pub render_fn: RenderFn,
    /// Number of operators in this chain
    pub n: usize,
    /// Index of the input buffer (modulation source)
    pub input_index: usize,
    /// Index of the output buffer (destination)
    pub output_index: usize,
}

impl Default for RenderCall {
    fn default() -> Self {
        Self {
            render_fn: render_operators::<1, -1, false>,
            n: 0,
            input_index: 0,
            output_index: 0,
        }
    }
}

struct RendererSpecs {
    n: usize,
    modulation_source: i32,
    additive: bool,
    render_fn: RenderFn,
}

// 6-operator opcodes (DX7)
#[rustfmt::skip]
static OPCODES_6: [[u8; 6]; 32] = [
    [ FB | out_flags(1), mod_flags(1) | out_flags(1), mod_flags(1) | out_flags(1), mod_flags(1) | OUTPUT, NO_MOD | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ NO_MOD | out_flags(1), mod_flags(1) | out_flags(1), mod_flags(1) | out_flags(1), mod_flags(1) | OUTPUT, FB | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ FB | out_flags(1), mod_flags(1) | out_flags(1), mod_flags(1) | OUTPUT, NO_MOD | out_flags(1), mod_flags(1) | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ FB_DST | NO_MOD | out_flags(1), mod_flags(1) | out_flags(1), FB_SRC | mod_flags(1) | OUTPUT, NO_MOD | out_flags(1), mod_flags(1) | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ FB | out_flags(1), mod_flags(1) | OUTPUT, NO_MOD | out_flags(1), mod_flags(1) | add_flags(0), NO_MOD | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ FB_DST | NO_MOD | out_flags(1), FB_SRC | mod_flags(1) | OUTPUT, NO_MOD | out_flags(1), mod_flags(1) | add_flags(0), NO_MOD | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ FB | out_flags(1), mod_flags(1) | out_flags(1), NO_MOD | add_flags(1), mod_flags(1) | OUTPUT, NO_MOD | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ NO_MOD | out_flags(1), mod_flags(1) | out_flags(1), FB | add_flags(1), mod_flags(1) | OUTPUT, NO_MOD | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ NO_MOD | out_flags(1), mod_flags(1) | out_flags(1), NO_MOD | add_flags(1), mod_flags(1) | OUTPUT, FB | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ NO_MOD | out_flags(1), NO_MOD | add_flags(1), mod_flags(1) | OUTPUT, FB | out_flags(1), mod_flags(1) | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ FB | out_flags(1), NO_MOD | add_flags(1), mod_flags(1) | OUTPUT, NO_MOD | out_flags(1), mod_flags(1) | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ NO_MOD | out_flags(1), NO_MOD | add_flags(1), NO_MOD | add_flags(1), mod_flags(1) | OUTPUT, FB | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ FB | out_flags(1), NO_MOD | add_flags(1), NO_MOD | add_flags(1), mod_flags(1) | OUTPUT, NO_MOD | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ FB | out_flags(1), NO_MOD | add_flags(1), mod_flags(1) | out_flags(1), mod_flags(1) | OUTPUT, NO_MOD | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ NO_MOD | out_flags(1), NO_MOD | add_flags(1), mod_flags(1) | out_flags(1), mod_flags(1) | OUTPUT, FB | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ FB | out_flags(1), mod_flags(1) | out_flags(1), NO_MOD | out_flags(2), mod_flags(2) | add_flags(1), NO_MOD | add_flags(1), mod_flags(1) | OUTPUT ],
    [ NO_MOD | out_flags(1), mod_flags(1) | out_flags(1), NO_MOD | out_flags(2), mod_flags(2) | add_flags(1), FB | add_flags(1), mod_flags(1) | OUTPUT ],
    [ NO_MOD | out_flags(1), mod_flags(1) | out_flags(1), mod_flags(1) | out_flags(1), FB | add_flags(1), NO_MOD | add_flags(1), mod_flags(1) | OUTPUT ],
    [ FB | out_flags(1), mod_flags(1) | OUTPUT, mod_flags(1) | add_flags(0), NO_MOD | out_flags(1), mod_flags(1) | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ NO_MOD | out_flags(1), NO_MOD | add_flags(1), mod_flags(1) | OUTPUT, FB | out_flags(1), mod_flags(1) | add_flags(0), mod_flags(1) | add_flags(0) ],
    [ NO_MOD | out_flags(1), mod_flags(1) | OUTPUT, mod_flags(1) | add_flags(0), FB | out_flags(1), mod_flags(1) | add_flags(0), mod_flags(1) | add_flags(0) ],
    [ FB | out_flags(1), mod_flags(1) | OUTPUT, mod_flags(1) | add_flags(0), mod_flags(1) | add_flags(0), NO_MOD | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ FB | out_flags(1), mod_flags(1) | OUTPUT, mod_flags(1) | add_flags(0), NO_MOD | out_flags(1), mod_flags(1) | add_flags(0), NO_MOD | add_flags(0) ],
    [ FB | out_flags(1), mod_flags(1) | OUTPUT, mod_flags(1) | add_flags(0), mod_flags(1) | add_flags(0), NO_MOD | add_flags(0), NO_MOD | add_flags(0) ],
    [ FB | out_flags(1), mod_flags(1) | OUTPUT, mod_flags(1) | add_flags(0), NO_MOD | add_flags(0), NO_MOD | add_flags(0), NO_MOD | add_flags(0) ],
    [ FB | out_flags(1), NO_MOD | add_flags(1), mod_flags(1) | OUTPUT, NO_MOD | out_flags(1), mod_flags(1) | add_flags(0), NO_MOD | add_flags(0) ],
    [ NO_MOD | out_flags(1), NO_MOD | add_flags(1), mod_flags(1) | OUTPUT, FB | out_flags(1), mod_flags(1) | add_flags(0), NO_MOD | add_flags(0) ],
    [ NO_MOD | OUTPUT, FB | out_flags(1), mod_flags(1) | out_flags(1), mod_flags(1) | add_flags(0), NO_MOD | out_flags(1), mod_flags(1) | add_flags(0) ],
    [ FB | out_flags(1), mod_flags(1) | OUTPUT, NO_MOD | out_flags(1), mod_flags(1) | add_flags(0), NO_MOD | add_flags(0), NO_MOD | add_flags(0) ],
    [ NO_MOD | OUTPUT, FB | out_flags(1), mod_flags(1) | out_flags(1), mod_flags(1) | add_flags(0), NO_MOD | add_flags(0), NO_MOD | add_flags(0) ],
    [ FB | out_flags(1), mod_flags(1) | OUTPUT, NO_MOD | add_flags(0), NO_MOD | add_flags(0), NO_MOD | add_flags(0), NO_MOD | add_flags(0) ],
    [ FB | OUTPUT, NO_MOD | add_flags(0), NO_MOD | add_flags(0), NO_MOD | add_flags(0), NO_MOD | add_flags(0), NO_MOD | add_flags(0) ],
];

// 6-operator renderers
static RENDERERS_6: [RendererSpecs; 9] = [
    RendererSpecs {
        n: 1,
        modulation_source: -2,
        additive: false,
        render_fn: render_operators::<1, -2, false>,
    },
    RendererSpecs {
        n: 1,
        modulation_source: -2,
        additive: true,
        render_fn: render_operators::<1, -2, true>,
    },
    RendererSpecs {
        n: 1,
        modulation_source: -1,
        additive: false,
        render_fn: render_operators::<1, -1, false>,
    },
    RendererSpecs {
        n: 1,
        modulation_source: -1,
        additive: true,
        render_fn: render_operators::<1, -1, true>,
    },
    RendererSpecs {
        n: 1,
        modulation_source: 0,
        additive: false,
        render_fn: render_operators::<1, 0, false>,
    },
    RendererSpecs {
        n: 1,
        modulation_source: 0,
        additive: true,
        render_fn: render_operators::<1, 0, true>,
    },
    RendererSpecs {
        n: 3,
        modulation_source: 2,
        additive: true,
        render_fn: render_operators::<3, 2, true>,
    },
    RendererSpecs {
        n: 2,
        modulation_source: 1,
        additive: true,
        render_fn: render_operators::<2, 1, true>,
    },
    RendererSpecs {
        n: 0,
        modulation_source: 0,
        additive: false,
        render_fn: render_operators::<1, -1, false>,
    },
];
