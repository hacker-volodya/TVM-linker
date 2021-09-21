/*
 * Copyright 2018-2021 TON DEV SOLUTIONS LTD.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific TON DEV software governing permissions and
 * limitations under the License.
 */

use ton_types::{Result, SliceData};

use super::types::{Instruction, Signaling, Quiet};
use super::loader::*;

pub(super) type LoadHandler = fn(&mut SliceData) -> Result<Instruction>;

#[derive(Clone, Copy)]
enum Handler {
    Direct(LoadHandler),
    Subset(usize),
}

pub struct Handlers {
    directs: [Handler; 256],
    subsets: Vec<Handlers>,
}

// adapted from ton-labs-vm/src/executor/engine/handlers.rs
impl Handlers {
    fn new() -> Handlers {
        Handlers {
            directs: [Handler::Direct(load_unknown); 256],
            subsets: Vec::new(),
        }
    }

    pub(super) fn new_code_page_0() -> Handlers {
        let mut handlers = Handlers::new();
        handlers
            .add_code_page_0_part_stack()
            .add_code_page_0_tuple()
            .add_code_page_0_part_constant()
            .add_code_page_0_arithmetic()
            .add_code_page_0_comparsion()
            .add_code_page_0_cell()
            .add_code_page_0_control_flow()
            .add_code_page_0_exceptions()
            .add_code_page_0_dictionaries()
            .add_code_page_0_gas_rand_config()
            .add_code_page_0_blockchain()
            .add_code_page_0_crypto()
            .add_code_page_0_debug()
            .add_subset(0xFF, Handlers::new()
                .set_range(0x00..0xF0, load_setcp)
                .set(0xF0, load_setcpx)
                .set_range(0xF1..0xFF, load_setcp)
                .set(0xFF, load_setcp)
            );
        handlers
    }

    fn add_code_page_0_part_stack(&mut self) -> &mut Handlers {
        self
            .set(0x00, load_nop)
            .set_range(0x01..0x10, load_xchg_simple)
            .set(0x10, load_xchg_std)
            .set(0x11, load_xchg_long)
            .set_range(0x12..0x20, load_xchg_simple)
            .set_range(0x20..0x30, load_push_simple)
            .set_range(0x30..0x40, load_pop_simple)
            .set_range(0x40..0x50, load_xchg3)
            .set(0x50, load_xchg2)
            .set(0x51, load_xcpu)
            .set(0x52, load_puxc)
            .set(0x53, load_push2)
            .add_subset(0x54, Handlers::new()
                .set_range(0x00..0x10, load_xchg3)
                .set_range(0x10..0x20, load_xc2pu)
                .set_range(0x20..0x30, load_xcpuxc)
                .set_range(0x30..0x40, load_xcpu2)
                .set_range(0x40..0x50, load_puxc2)
                .set_range(0x50..0x60, load_puxcpu)
                .set_range(0x60..0x70, load_pu2xc)
                .set_range(0x70..0x80, load_push3)
            )
            .set(0x55, load_blkswap)
            .set(0x56, load_push)
            .set(0x57, load_pop)
            .set(0x58, load_rot)
            .set(0x59, load_rotrev)
            .set(0x5A, load_swap2)
            .set(0x5B, load_drop2)
            .set(0x5C, load_dup2)
            .set(0x5D, load_over2)
            .set(0x5E, load_reverse)
            .add_subset(0x5F, Handlers::new()
                .set_range(0x00..0x10, load_blkdrop)
                .set_range(0x10..0xFF, load_blkpush)
                .set(0xFF, load_blkpush)
            )
            .set(0x60, load_pick)
            .set(0x61, load_rollx)
            .set(0x62, load_rollrevx)
            .set(0x63, load_blkswx)
            .set(0x64, load_revx)
            .set(0x65, load_dropx)
            .set(0x66, load_tuck)
            .set(0x67, load_xchgx)
            .set(0x68, load_depth)
            .set(0x69, load_chkdepth)
            .set(0x6A, load_onlytopx)
            .set(0x6B, load_onlyx)
            .add_subset(0x6C, Handlers::new()
                .set_range(0x10..0xFF, load_blkdrop2)
                .set(0xFF, load_blkdrop2)
            )
    }

    fn add_code_page_0_tuple(&mut self) -> &mut Handlers {
        self
            .set(0x6D, load_null)
            .set(0x6E, load_isnull)
            .add_subset(0x6F, Handlers::new()
                .set_range(0x00..0x10, load_tuple_create)
                .set_range(0x10..0x20, load_tuple_index)
                .set_range(0x20..0x30, load_tuple_un)
                .set_range(0x30..0x40, load_tuple_unpackfirst)
                .set_range(0x40..0x50, load_tuple_explode)
                .set_range(0x50..0x60, load_tuple_setindex)
                .set_range(0x60..0x70, load_tuple_index_quiet)
                .set_range(0x70..0x80, load_tuple_setindex_quiet)
                .set(0x80, load_tuple_createvar)
                .set(0x81, load_tuple_indexvar)
                .set(0x82, load_tuple_untuplevar)
                .set(0x83, load_tuple_unpackfirstvar)
                .set(0x84, load_tuple_explodevar)
                .set(0x85, load_tuple_setindexvar)
                .set(0x86, load_tuple_indexvar_quiet)
                .set(0x87, load_tuple_setindexvar_quiet)
                .set(0x88, load_tuple_len)
                .set(0x89, load_tuple_len_quiet)
                .set(0x8A, load_istuple)
                .set(0x8B, load_tuple_last)
                .set(0x8C, load_tuple_push)
                .set(0x8D, load_tuple_pop)
                .set(0xA0, load_nullswapif)
                .set(0xA1, load_nullswapifnot)
                .set(0xA2, load_nullrotrif)
                .set(0xA3, load_nullrotrifnot)
                .set(0xA4, load_nullswapif2)
                .set(0xA5, load_nullswapifnot2)
                .set(0xA6, load_nullrotrif2)
                .set(0xA7, load_nullrotrifnot2)
                .set_range(0xB0..0xC0, load_tuple_index2)
                .set_range(0xC0..0xFF, load_tuple_index3)
                .set(0xFF, load_tuple_index3)
            )
    }

    fn add_code_page_0_part_constant(&mut self) -> &mut Handlers {
        self
            .set_range(0x70..0x82, load_pushint)
            .set(0x82, load_pushint_big)
            .add_subset(0x83, Handlers::new()
                .set_range(0x00..0xFF, load_pushpow2)
                .set(0xFF, load_pushnan)
            )
            .set(0x84, load_pushpow2dec)
            .set(0x85, load_pushnegpow2)
            .set(0x88, load_pushref)
            .set(0x89, load_pushrefslice)
            .set(0x8A, load_pushrefcont)
            .set(0x8B, load_pushslice_short)
            .set(0x8C, load_pushslice_mid)
            .set(0x8D, load_pushslice_long)
            .set_range(0x8E..0x90, load_pushcont_long)
            .set_range(0x90..0xA0, load_pushcont_short)
    }

    fn add_code_page_0_arithmetic(&mut self) -> &mut Handlers {
        self
            .set(0xA0, load_add::<Signaling>)
            .set(0xA1, load_sub::<Signaling>)
            .set(0xA2, load_subr::<Signaling>)
            .set(0xA3, load_negate::<Signaling>)
            .set(0xA4, load_inc::<Signaling>)
            .set(0xA5, load_dec::<Signaling>)
            .set(0xA6, load_addconst::<Signaling>)
            .set(0xA7, load_mulconst::<Signaling>)
            .set(0xA8, load_mul::<Signaling>)
            .set(0xA9, load_divmod::<Signaling>)
            .set(0xAA, load_lshift::<Signaling>)
            .set(0xAB, load_rshift::<Signaling>)
            .set(0xAC, load_lshift::<Signaling>)
            .set(0xAD, load_rshift::<Signaling>)
            .set(0xAE, load_pow2::<Signaling>)
            .set(0xB0, load_and::<Signaling>)
            .set(0xB1, load_or::<Signaling>)
            .set(0xB2, load_xor::<Signaling>)
            .set(0xB3, load_not::<Signaling>)
            .set(0xB4, load_fits::<Signaling>)
            .set(0xB5, load_ufits::<Signaling>)
            .add_subset(0xB6, Handlers::new()
                .set(0x00, load_fitsx::<Signaling>)
                .set(0x01, load_ufitsx::<Signaling>)
                .set(0x02, load_bitsize::<Signaling>)
                .set(0x03, load_ubitsize::<Signaling>)
                .set(0x08, load_min::<Signaling>)
                .set(0x09, load_max::<Signaling>)
                .set(0x0A, load_minmax::<Signaling>)
                .set(0x0B, load_abs::<Signaling>)
            )
            .add_subset(0xB7, Handlers::new()
                .set(0xA0, load_add::<Quiet>)
                .set(0xA1, load_sub::<Quiet>)
                .set(0xA2, load_subr::<Quiet>)
                .set(0xA3, load_negate::<Quiet>)
                .set(0xA4, load_inc::<Quiet>)
                .set(0xA5, load_dec::<Quiet>)
                .set(0xA6, load_addconst::<Quiet>)
                .set(0xA7, load_mulconst::<Quiet>)
                .set(0xA8, load_mul::<Quiet>)
                .set(0xA9, load_divmod::<Quiet>)
                .set(0xAA, load_lshift::<Quiet>)
                .set(0xAB, load_rshift::<Quiet>)
                .set(0xAC, load_lshift::<Quiet>)
                .set(0xAD, load_rshift::<Quiet>)
                .set(0xAE, load_pow2::<Quiet>)
                .set(0xB0, load_and::<Quiet>)
                .set(0xB1, load_or::<Quiet>)
                .set(0xB2, load_xor::<Quiet>)
                .set(0xB3, load_not::<Quiet>)
                .set(0xB4, load_fits::<Quiet>)
                .set(0xB5, load_ufits::<Quiet>)
                .add_subset(0xB6, Handlers::new()
                    .set(0x00, load_fitsx::<Quiet>)
                    .set(0x01, load_ufitsx::<Quiet>)
                    .set(0x02, load_bitsize::<Quiet>)
                    .set(0x03, load_ubitsize::<Quiet>)
                    .set(0x08, load_min::<Quiet>)
                    .set(0x09, load_max::<Quiet>)
                    .set(0x0A, load_minmax::<Quiet>)
                    .set(0x0B, load_abs::<Quiet>)
                )
                .set(0xB8, load_sgn::<Quiet>)
                .set(0xB9, load_less::<Quiet>)
                .set(0xBA, load_equal::<Quiet>)
                .set(0xBB, load_leq::<Quiet>)
                .set(0xBC, load_greater::<Quiet>)
                .set(0xBD, load_neq::<Quiet>)
                .set(0xBE, load_geq::<Quiet>)
                .set(0xBF, load_cmp::<Quiet>)
                .set(0xC0, load_eqint::<Quiet>)
                .set(0xC1, load_lessint::<Quiet>)
                .set(0xC2, load_gtint::<Quiet>)
                .set(0xC3, load_neqint::<Quiet>)
            )
    }

    fn add_code_page_0_comparsion(&mut self) -> &mut Handlers {
        self
            .set(0xB8, load_sgn::<Signaling>)
            .set(0xB9, load_less::<Signaling>)
            .set(0xBA, load_equal::<Signaling>)
            .set(0xBB, load_leq::<Signaling>)
            .set(0xBC, load_greater::<Signaling>)
            .set(0xBD, load_neq::<Signaling>)
            .set(0xBE, load_geq::<Signaling>)
            .set(0xBF, load_cmp::<Signaling>)
            .set(0xC0, load_eqint::<Signaling>)
            .set(0xC1, load_lessint::<Signaling>)
            .set(0xC2, load_gtint::<Signaling>)
            .set(0xC3, load_neqint::<Signaling>)
            .set(0xC4, load_isnan)
            .set(0xC5, load_chknan)
            .add_subset(0xC7, Handlers::new()
                .set(0x00, load_sempty)
                .set(0x01, load_sdempty)
                .set(0x02, load_srempty)
                .set(0x03, load_sdfirst)
                .set(0x04, load_sdlexcmp)
                .set(0x05, load_sdeq)
                .set(0x08, load_sdpfx)
                .set(0x09, load_sdpfxrev)
                .set(0x0A, load_sdppfx)
                .set(0x0B, load_sdppfxrev)
                .set(0x0C, load_sdsfx)
                .set(0x0D, load_sdsfxrev)
                .set(0x0E, load_sdpsfx)
                .set(0x0F, load_sdpsfxrev)
                .set(0x10, load_sdcntlead0)
                .set(0x11, load_sdcntlead1)
                .set(0x12, load_sdcnttrail0)
                .set(0x13, load_sdcnttrail1)
            )
    }

    fn add_code_page_0_cell(&mut self) -> &mut Handlers {
        self
            .set(0xC8, load_newc)
            .set(0xC9, load_endc)
            .set(0xCA, load_sti)
            .set(0xCB, load_stu)
            .set(0xCC, load_stref)
            .set(0xCD, load_endcst)
            .set(0xCE, load_stslice)
            .add_subset(0xCF, Handlers::new()
                .set(0x00, load_stix)
                .set(0x01, load_stux)
                .set(0x02, load_stixr)
                .set(0x03, load_stuxr)
                .set(0x04, load_stixq)
                .set(0x05, load_stuxq)
                .set(0x06, load_stixrq)
                .set(0x07, load_stuxrq)
                .set(0x08, load_sti)
                .set(0x09, load_stu)
                .set(0x0A, load_stir)
                .set(0x0B, load_stur)
                .set(0x0C, load_stiq)
                .set(0x0D, load_stuq)
                .set(0x0E, load_stirq)
                .set(0x0F, load_sturq)
                .set(0x10, load_stref)
                .set(0x11, load_stbref)
                .set(0x12, load_stslice)
                .set(0x13, load_stb)
                .set(0x14, load_strefr)
                .set(0x15, load_endcst)
                .set(0x16, load_stslicer)
                .set(0x17, load_stbr)
                .set(0x18, load_strefq)
                .set(0x19, load_stbrefq)
                .set(0x1A, load_stsliceq)
                .set(0x1B, load_stbq)
                .set(0x1C, load_strefrq)
                .set(0x1D, load_stbrefrq)
                .set(0x1E, load_stslicerq)
                .set(0x1F, load_stbrq)
                .set(0x20, load_strefconst)
                .set(0x21, load_stref2const)
                .set(0x23, load_endxc)
                .set(0x28, load_stile4)
                .set(0x29, load_stule4)
                .set(0x2A, load_stile8)
                .set(0x2B, load_stule8)
                .set(0x30, load_bdepth)
                .set(0x31, load_bbits)
                .set(0x32, load_brefs)
                .set(0x33, load_bbitrefs)
                .set(0x35, load_brembits)
                .set(0x36, load_bremrefs)
                .set(0x37, load_brembitrefs)
                .set(0x38, load_bchkbits_short)
                .set(0x39, load_bchkbits_long)
                .set(0x3A, load_bchkrefs)
                .set(0x3B, load_bchkbitrefs)
                .set(0x3C, load_bchkbitsq_short)
                .set(0x3D, load_bchkbitsq_long)
                .set(0x3E, load_bchkrefsq)
                .set(0x3F, load_bchkbitrefsq)
                .set(0x40, load_stzeroes)
                .set(0x41, load_stones)
                .set(0x42, load_stsame)
                .set_range(0x80..0xFF, load_stsliceconst)
                .set(0xFF, load_stsliceconst)
            )
            .set(0xD0, load_ctos)
            .set(0xD1, load_ends)
            .set(0xD2, load_ldi)
            .set(0xD3, load_ldu)
            .set(0xD4, load_ldref)
            .set(0xD5, load_ldrefrtos)
            .set(0xD6, load_ldslice)
            .add_subset(0xD7, Handlers::new()
                .set(0x00, load_ldix)
                .set(0x01, load_ldux)
                .set(0x02, load_pldix)
                .set(0x03, load_pldux)
                .set(0x04, load_ldixq)
                .set(0x05, load_lduxq)
                .set(0x06, load_pldixq)
                .set(0x07, load_plduxq)
                .set(0x08, load_ldi)
                .set(0x09, load_ldu)
                .set(0x0A, load_pldi)
                .set(0x0B, load_pldu)
                .set(0x0C, load_ldiq)
                .set(0x0D, load_lduq)
                .set(0x0E, load_pldiq)
                .set(0x0F, load_plduq)
                .set_range(0x10..0x18, load_plduz)
                .set(0x18, load_ldslicex)
                .set(0x19, load_pldslicex)
                .set(0x1A, load_ldslicexq)
                .set(0x1B, load_pldslicexq)
                .set(0x1C, load_ldslice)
                .set(0x1D, load_pldslice)
                .set(0x1E, load_ldsliceq)
                .set(0x1F, load_pldsliceq)
                .set(0x20, load_pldslicex)
                .set(0x21, load_sdskipfirst)
                .set(0x22, load_sdcutlast)
                .set(0x23, load_sdskiplast)
                .set(0x24, load_sdsubstr)
                .set(0x26, load_sdbeginsx)
                .set(0x27, load_sdbeginsxq)
                .set_range(0x28..0x2C, load_sdbegins)
                .set_range(0x2C..0x30, load_sdbeginsq)
                .set(0x30, load_scutfirst)
                .set(0x31, load_sskipfirst)
                .set(0x32, load_scutlast)
                .set(0x33, load_sskiplast)
                .set(0x34, load_subslice)
                .set(0x36, load_split)
                .set(0x37, load_splitq)
                .set(0x39, load_xctos)
                .set(0x3A, load_xload)
                .set(0x3B, load_xloadq)
                .set(0x41, load_schkbits)
                .set(0x42, load_schkrefs)
                .set(0x43, load_schkbitrefs)
                .set(0x45, load_schkbitsq)
                .set(0x46, load_schkrefsq)
                .set(0x47, load_schkbitrefsq)
                .set(0x48, load_pldrefvar)
                .set(0x49, load_sbits)
                .set(0x4A, load_srefs)
                .set(0x4B, load_sbitrefs)
                .set(0x4C, load_pldref)
                .set_range(0x4D..0x50, load_pldrefidx)
                .set(0x50, load_ldile4)
                .set(0x51, load_ldule4)
                .set(0x52, load_ldile8)
                .set(0x53, load_ldule8)
                .set(0x54, load_pldile4)
                .set(0x55, load_pldule4)
                .set(0x56, load_pldile8)
                .set(0x57, load_pldule8)
                .set(0x58, load_ldile4q)
                .set(0x59, load_ldule4q)
                .set(0x5A, load_ldile8q)
                .set(0x5B, load_ldule8q)
                .set(0x5C, load_pldile4q)
                .set(0x5D, load_pldule4q)
                .set(0x5E, load_pldile8q)
                .set(0x5F, load_pldule8q)
                .set(0x60, load_ldzeroes)
                .set(0x61, load_ldones)
                .set(0x62, load_ldsame)
                .set(0x64, load_sdepth)
                .set(0x65, load_cdepth)
            )
    }

    fn add_code_page_0_control_flow(&mut self) -> &mut Handlers {
        self
            .set(0xD8, load_callx)
            .set(0xD9, load_jmpx)
            .set(0xDA, load_callxargs)
            .add_subset(0xDB, Handlers::new()
                .set_range(0x00..0x10, load_callxargs)
                .set_range(0x10..0x20, load_jmpxargs)
                .set_range(0x20..0x30, load_retargs)
                .set(0x30, load_ret)
                .set(0x31, load_retalt)
                .set(0x32, load_retbool)
                .set(0x34, load_callcc)
                .set(0x35, load_jmpxdata)
                .set(0x36, load_callccargs)
                .set(0x38, load_callxva)
                .set(0x39, load_retva)
                .set(0x3A, load_jmpxva)
                .set(0x3B, load_callccva)
                .set(0x3C, load_callref)
                .set(0x3D, load_jmpref)
                .set(0x3E, load_jmprefdata)
                .set(0x3F, load_retdata)
            )
            .set(0xDE, load_if)
            .set(0xDC, load_ifret)
            .set(0xDD, load_ifnotret)
            .set(0xDF, load_ifnot)
            .set(0xE0, load_ifjmp)
            .set(0xE1, load_ifnotjmp)
            .set(0xE2, load_ifelse)
            .add_subset(0xE3, Handlers::new()
                .set(0x00, load_ifref)
                .set(0x01, load_ifnotref)
                .set(0x02, load_ifjmpref)
                .set(0x03, load_ifnotjmpref)
                .set(0x04, load_condsel)
                .set(0x05, load_condselchk)
                .set(0x08, load_ifretalt)
                .set(0x09, load_ifnotretalt)
                .set(0x0D, load_ifrefelse)
                .set(0x0E, load_ifelseref)
                .set(0x0F, load_ifrefelseref)
                .set(0x14, load_repeat_break)
                .set(0x15, load_repeatend_break)
                .set(0x16, load_until_break)
                .set(0x17, load_untilend_break)
                .set(0x18, load_while_break)
                .set(0x19, load_whileend_break)
                .set(0x1A, load_again_break)
                .set(0x1B, load_againend_break)
                .set_range(0x80..0xA0, load_ifbitjmp)
                .set_range(0xA0..0xC0, load_ifnbitjmp)
                .set_range(0xC0..0xE0, load_ifbitjmpref)
                .set_range(0xE0..0xFF, load_ifnbitjmpref)
                .set(0xFF, load_ifnbitjmpref)
             )
            .set(0xE4, load_repeat)
            .set(0xE5, load_repeatend)
            .set(0xE6, load_until)
            .set(0xE7, load_untilend)
            .set(0xE8, load_while)
            .set(0xE9, load_whileend)
            .set(0xEA, load_again)
            .set(0xEB, load_againend)
            .set(0xEC, load_setcontargs)
            .add_subset(0xED, Handlers::new()
                .set_range(0x00..0x10, load_returnargs)
                .set(0x10, load_returnva)
                .set(0x11, load_setcontva)
                .set(0x12, load_setnumva)
                .set(0x1E, load_bless)
                .set(0x1F, load_blessva)
                .set_range(0x40..0x50, load_pushctr)
                .set_range(0x50..0x60, load_popctr)
                .set_range(0x60..0x70, load_setcontctr)
                .set_range(0x70..0x80, load_setretctr)
                .set_range(0x80..0x90, load_setaltctr)
                .set_range(0x90..0xA0, load_popsave)
                .set_range(0xA0..0xB0, load_save)
                .set_range(0xB0..0xC0, load_savealt)
                .set_range(0xC0..0xD0, load_saveboth)
                .set(0xE0, load_pushctrx)
                .set(0xE1, load_popctrx)
                .set(0xE2, load_setcontctrx)
                .set(0xF0, load_compos)
                .set(0xF1, load_composalt)
                .set(0xF2, load_composboth)
                .set(0xF3, load_atexit)
                .set(0xF4, load_atexitalt)
                .set(0xF5, load_setexitalt)
                .set(0xF6, load_thenret)
                .set(0xF7, load_thenretalt)
                .set(0xF8, load_invert)
                .set(0xF9, load_booleval)
                .set(0xFA, load_samealt)
                .set(0xFB, load_samealt_save)
            )
            .set(0xEE, load_blessargs)
            .set(0xF0, load_call_short)
            .add_subset(0xF1, Handlers::new()
                .set_range(0x00..0x40, load_call_long)
                .set_range(0x40..0x80, load_jmp)
                .set_range(0x80..0xC0, load_prepare)
            )
    }

    fn add_code_page_0_exceptions(&mut self) -> &mut Handlers {
        self
            .add_subset(0xF2, Handlers::new()
                .set_range(0x00..0x40, load_throw_short)
                .set_range(0x40..0x80, load_throwif_short)
                .set_range(0x80..0xC0, load_throwifnot_short)
                .set_range(0xC0..0xC8, load_throw_long)
                .set_range(0xC8..0xD0, load_throwarg)
                .set_range(0xD0..0xD8, load_throwif_long)
                .set_range(0xD8..0xE0, load_throwargif)
                .set_range(0xE0..0xE8, load_throwifnot_long)
                .set_range(0xE8..0xF0, load_throwargifnot)
                .set(0xF0, load_throwany)
                .set(0xF1, load_throwargany)
                .set(0xF2, load_throwanyif)
                .set(0xF3, load_throwarganyif)
                .set(0xF4, load_throwanyifnot)
                .set(0xF5, load_throwarganyifnot)
                .set(0xFF, load_try)
            )
            .set(0xF3, load_tryargs)
    }

    fn add_code_page_0_blockchain(&mut self) -> &mut Handlers {
        self
            .add_subset(0xFA, Handlers::new()
                .set(0x00, load_ldgrams)
                .set(0x01, load_ldvarint16)
                .set(0x02, load_stgrams)
                .set(0x03, load_stvarint16)
                .set(0x04, load_ldvaruint32)
                .set(0x05, load_ldvarint32)
                .set(0x06, load_stvaruint32)
                .set(0x07, load_stvarint32)
                .set(0x40, load_ldmsgaddr::<Signaling>)
                .set(0x41, load_ldmsgaddr::<Quiet>)
                .set(0x42, load_parsemsgaddr::<Signaling>)
                .set(0x43, load_parsemsgaddr::<Quiet>)
                .set(0x44, load_rewrite_std_addr::<Signaling>)
                .set(0x45, load_rewrite_std_addr::<Quiet>)
                .set(0x46, load_rewrite_var_addr::<Signaling>)
                .set(0x47, load_rewrite_var_addr::<Quiet>)
            )
            .add_subset(0xFB, Handlers::new()
                .set(0x00, load_sendrawmsg)
                .set(0x02, load_rawreserve)
                .set(0x03, load_rawreservex)
                .set(0x04, load_setcode)
                .set(0x06, load_setlibcode)
                .set(0x07, load_changelib)
            )
    }

    fn add_code_page_0_dictionaries(&mut self) -> &mut Handlers {
        self
            .add_subset(0xF4, Handlers::new()
                .set(0x00, load_stdict)
                .set(0x01, load_skipdict)
                .set(0x02, load_lddicts)
                .set(0x03, load_plddicts)
                .set(0x04, load_lddict)
                .set(0x05, load_plddict)
                .set(0x06, load_lddictq)
                .set(0x07, load_plddictq)
                .set(0x0A, load_dictget)
                .set(0x0B, load_dictgetref)
                .set(0x0C, load_dictiget)
                .set(0x0D, load_dictigetref)
                .set(0x0E, load_dictuget)
                .set(0x0F, load_dictugetref)
                .set(0x12, load_dictset)
                .set(0x13, load_dictsetref)
                .set(0x14, load_dictiset)
                .set(0x15, load_dictisetref)
                .set(0x16, load_dictuset)
                .set(0x17, load_dictusetref)
                .set(0x1A, load_dictsetget)
                .set(0x1B, load_dictsetgetref)
                .set(0x1C, load_dictisetget)
                .set(0x1D, load_dictisetgetref)
                .set(0x1E, load_dictusetget)
                .set(0x1F, load_dictusetgetref)
                .set(0x22, load_dictreplace)
                .set(0x23, load_dictreplaceref)
                .set(0x24, load_dictireplace)
                .set(0x25, load_dictireplaceref)
                .set(0x26, load_dictureplace)
                .set(0x27, load_dictureplaceref)
                .set(0x2A, load_dictreplaceget)
                .set(0x2B, load_dictreplacegetref)
                .set(0x2C, load_dictireplaceget)
                .set(0x2D, load_dictireplacegetref)
                .set(0x2E, load_dictureplaceget)
                .set(0x2F, load_dictureplacegetref)
                .set(0x32, load_dictadd)
                .set(0x33, load_dictaddref)
                .set(0x34, load_dictiadd)
                .set(0x35, load_dictiaddref)
                .set(0x36, load_dictuadd)
                .set(0x37, load_dictuaddref)
                .set(0x3A, load_dictaddget)
                .set(0x3B, load_dictaddgetref)
                .set(0x3C, load_dictiaddget)
                .set(0x3D, load_dictiaddgetref)
                .set(0x3E, load_dictuaddget)
                .set(0x3F, load_dictuaddgetref)
                .set(0x41, load_dictsetb)
                .set(0x42, load_dictisetb)
                .set(0x43, load_dictusetb)
                .set(0x45, load_dictsetgetb)
                .set(0x46, load_dictisetgetb)
                .set(0x47, load_dictusetgetb)
                .set(0x49, load_dictreplaceb)
                .set(0x4A, load_dictireplaceb)
                .set(0x4B, load_dictureplaceb)
                .set(0x4D, load_dictreplacegetb)
                .set(0x4E, load_dictireplacegetb)
                .set(0x4F, load_dictureplacegetb)
                .set(0x51, load_dictaddb)
                .set(0x52, load_dictiaddb)
                .set(0x53, load_dictuaddb)
                .set(0x55, load_dictaddgetb)
                .set(0x56, load_dictiaddgetb)
                .set(0x57, load_dictuaddgetb)
                .set(0x59, load_dictdel)
                .set(0x5A, load_dictidel)
                .set(0x5B, load_dictudel)
                .set(0x62, load_dictdelget)
                .set(0x63, load_dictdelgetref)
                .set(0x64, load_dictidelget)
                .set(0x65, load_dictidelgetref)
                .set(0x66, load_dictudelget)
                .set(0x67, load_dictudelgetref)
                .set(0x69, load_dictgetoptref)
                .set(0x6A, load_dictigetoptref)
                .set(0x6B, load_dictugetoptref)
                .set(0x6D, load_dictsetgetoptref)
                .set(0x6E, load_dictisetgetoptref)
                .set(0x6F, load_dictusetgetoptref)
                .set(0x70, load_pfxdictset)
                .set(0x71, load_pfxdictreplace)
                .set(0x72, load_pfxdictadd)
                .set(0x73, load_pfxdictdel)
                .set(0x74, load_dictgetnext)
                .set(0x75, load_dictgetnexteq)
                .set(0x76, load_dictgetprev)
                .set(0x77, load_dictgetpreveq)
                .set(0x78, load_dictigetnext)
                .set(0x79, load_dictigetnexteq)
                .set(0x7A, load_dictigetprev)
                .set(0x7B, load_dictigetpreveq)
                .set(0x7C, load_dictugetnext)
                .set(0x7D, load_dictugetnexteq)
                .set(0x7E, load_dictugetprev)
                .set(0x7F, load_dictugetpreveq)
                .set(0x82, load_dictmin)
                .set(0x83, load_dictminref)
                .set(0x84, load_dictimin)
                .set(0x85, load_dictiminref)
                .set(0x86, load_dictumin)
                .set(0x87, load_dictuminref)
                .set(0x8A, load_dictmax)
                .set(0x8B, load_dictmaxref)
                .set(0x8C, load_dictimax)
                .set(0x8D, load_dictimaxref)
                .set(0x8E, load_dictumax)
                .set(0x8F, load_dictumaxref)
                .set(0x92, load_dictremmin)
                .set(0x93, load_dictremminref)
                .set(0x94, load_dictiremmin)
                .set(0x95, load_dictiremminref)
                .set(0x96, load_dicturemmin)
                .set(0x97, load_dicturemminref)
                .set(0x9A, load_dictremmax)
                .set(0x9B, load_dictremmaxref)
                .set(0x9C, load_dictiremmax)
                .set(0x9D, load_dictiremmaxref)
                .set(0x9E, load_dicturemmax)
                .set(0x9F, load_dicturemmaxref)
                .set(0xA0, load_dictigetjmp)
                .set(0xA1, load_dictugetjmp)
                .set(0xA2, load_dictigetexec)
                .set(0xA3, load_dictugetexec)
                .set_range(0xA4..0xA8, load_dictpushconst)
                .set(0xA8, load_pfxdictgetq)
                .set(0xA9, load_pfxdictget)
                .set(0xAA, load_pfxdictgetjmp)
                .set(0xAB, load_pfxdictgetexec)
                .set_range(0xAC..0xAF, load_pfxdictswitch)
                .set(0xAF, load_pfxdictswitch)
                .set(0xB1, load_subdictget)
                .set(0xB2, load_subdictiget)
                .set(0xB3, load_subdictuget)
                .set(0xB5, load_subdictrpget)
                .set(0xB6, load_subdictirpget)
                .set(0xB7, load_subdicturpget)
                .set(0xBC, load_dictigetjmpz)
                .set(0xBD, load_dictugetjmpz)
                .set(0xBE, load_dictigetexecz)
                .set(0xBF, load_dictugetexecz)
            )
    }

    fn add_code_page_0_gas_rand_config(&mut self) -> &mut Handlers {
        self
            .add_subset(0xF8, Handlers::new()
                .set(0x00, load_accept)
                .set(0x01, load_setgaslimit)
                .set(0x02, load_buygas)
                .set(0x04, load_gramtogas)
                .set(0x05, load_gastogram)
                .set(0x0F, load_commit)
                .set(0x10, load_randu256)
                .set(0x11, load_rand)
                .set(0x14, load_setrand)
                .set(0x15, load_addrand)
                .set(0x20, load_getparam)
                .set(0x21, load_getparam)
                .set(0x22, load_getparam)
                .set(0x23, load_now)
                .set(0x24, load_blocklt)
                .set(0x25, load_ltime)
                .set(0x26, load_randseed)
                .set(0x27, load_balance)
                .set(0x28, load_my_addr)
                .set(0x29, load_config_root)
                .set(0x30, load_config_dict)
                .set(0x32, load_config_ref_param)
                .set(0x33, load_config_opt_param)
                .set(0x40, load_getglobvar)
                .set_range(0x41..0x5F, load_getglob)
                .set(0x5F, load_getglob)
                .set(0x60, load_setglobvar)
                .set_range(0x61..0x7F, load_setglob)
                .set(0x7F, load_setglob)
            )
    }

    fn add_code_page_0_crypto(&mut self) -> &mut Handlers {
        self
        .add_subset(0xF9, Handlers::new()
            .set(0x00, load_hashcu)
            .set(0x01, load_hashsu)
            .set(0x02, load_sha256u)
            .set(0x10, load_chksignu)
            .set(0x11, load_chksigns)
            .set(0x40, load_cdatasizeq)
            .set(0x41, load_cdatasize)
            .set(0x42, load_sdatasizeq)
            .set(0x43, load_sdatasize)
        )
    }

    fn add_code_page_0_debug(&mut self) -> &mut Handlers {
        self.add_subset(0xFE, Handlers::new()
            .set(0x00, load_dump_stack)
            .set_range(0x01..0x0F, load_dump_stack_top)
            .set(0x10, load_dump_hex)
            .set(0x11, load_print_hex)
            .set(0x12, load_dump_bin)
            .set(0x13, load_print_bin)
            .set(0x14, load_dump_str)
            .set(0x15, load_print_str)
            .set(0x1E, load_debug_off)
            .set(0x1F, load_debug_on)
            .set_range(0x20..0x2F, load_dump_var)
            .set_range(0x30..0x3F, load_print_var)
            .set_range(0xF0..0xFF, load_dump_string)
            .set(0xFF, load_dump_string)
        )
    }

    pub(crate) fn get_handler(&self, slice: &mut SliceData) -> Result<LoadHandler> {
        let cmd = slice.get_next_byte()?;
        match self.directs[cmd as usize] {
            Handler::Direct(handler) => Ok(handler),
            Handler::Subset(i) => self.subsets[i].get_handler(slice),
        }
    }

    fn add_subset(&mut self, code: u8, subset: &mut Handlers) -> &mut Handlers {
        match self.directs[code as usize] {
            Handler::Direct(x) => if x as usize == load_unknown as usize {
                self.directs[code as usize] = Handler::Subset(self.subsets.len());
                self.subsets.push(std::mem::replace(subset, Handlers::new()))
            } else {
                panic!("Slot for subset {:02x} is already occupied", code)
            },
            _ => panic!("Subset {:02x} is already registered", code),
        }
        self
    }

    fn register_handler(&mut self, code: u8, handler: LoadHandler) {
        match self.directs[code as usize] {
            Handler::Direct(x) => if x as usize == load_unknown as usize {
                self.directs[code as usize] = Handler::Direct(handler)
            } else {
                panic!("Code {:02x} is already registered", code)
            },
            _ => panic!("Slot for code {:02x} is already occupied", code),
        }
    }

    fn set(&mut self, code: u8, handler: LoadHandler) -> &mut Handlers {
        self.register_handler(code, handler);
        self
    }

    fn set_range(&mut self, codes: std::ops::Range<u8>, handler: LoadHandler) -> &mut Handlers {
        for code in codes {
            self.register_handler(code, handler);
        }
        self
    }
}
