#![allow(
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::doc_markdown,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_sign_loss,
    clippy::items_after_statements,
    clippy::wildcard_enum_match_arm,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::return_self_not_must_use
)]

use alloc::boxed::Box;

pub mod common;

// Vendor-organized mappers
pub mod bandai;
pub mod camerica;
pub mod irem;
pub mod jaleco;
pub mod konami;
pub mod namco;
pub mod nintendo;
pub mod sunsoft;
pub mod taito;
pub mod unlicensed;

pub trait MapperImpl {
    fn cpu_read(&mut self, addr: u16) -> u8;
    fn cpu_write(&mut self, addr: u16, val: u8);
    fn ppu_read(&mut self, addr: u16) -> u8;
    fn ppu_write(&mut self, addr: u16, val: u8);
    fn mirroring(&self) -> u8;
    fn irq_pending(&self) -> bool;
    fn ack_irq(&mut self);
    fn clock_scanline(&mut self) {}

    fn has_chr_ram(&self) -> bool;

    // MMC5-specific extensions with default no-op implementations:

    /// Called once per scanline by the PPU (for MMC5 scanline IRQ).
    fn notify_scanline(&mut self, _scanline: u16) {}

    /// Returns the nametable mapping register value.
    /// 0xFF = standard mirroring (use `mirroring()`).
    /// Otherwise, each 2-bit pair maps NT0-3 to source (0=`CIRAM_A`, 1=`CIRAM_B`, 2=`ExRAM`, 3=Fill).
    fn nt_mapping(&self) -> u8 {
        0xFF
    }

    /// Read from a non-CIRAM nametable source (`ExRAM` or fill mode).
    fn read_nt_ext(&mut self, _addr: u16, _nt_source: u8) -> u8 {
        0
    }

    /// Write to a non-CIRAM nametable source (`ExRAM`).
    fn write_nt_ext(&mut self, _addr: u16, _nt_source: u8, _val: u8) {}

    /// Set CHR fetch mode to background (for `ExRAM` mode 1).
    fn set_chr_fetch_bg(&mut self) {}

    /// Set CHR fetch mode to sprite (for `ExRAM` mode 1).
    fn set_chr_fetch_sprite(&mut self) {}

    /// Set the extended CHR bank from `ExRAM` byte (for `ExRAM` mode 1).
    fn set_extended_chr_bank(&mut self, _bank: u8) {}

    /// Get the extended CHR bank.
    fn get_extended_chr_bank(&self) -> u8 {
        0
    }

    /// Get the `ExRAM` mode.
    fn get_ex_ram_mode(&self) -> u8 {
        0
    }

    /// Get fill mode tile value.
    fn get_fill_tile(&self) -> u8 {
        0
    }

    /// Get fill mode attribute value.
    fn get_fill_attr(&self) -> u8 {
        0
    }

    /// Read a byte from `ExRAM` at the given offset.
    fn read_ex_ram_byte(&mut self, _offset: u16) -> u8 {
        0
    }
}

pub enum Mapper {
    // Nintendo
    Nrom(Box<nintendo::Nrom>),
    UxRom(Box<nintendo::UxRom>),
    Cnrom(Box<nintendo::Cnrom>),
    Mmc1(Box<nintendo::Mmc1>),
    Mmc2(Box<nintendo::Mmc2>),
    Mmc3(Box<nintendo::Mmc3>),
    Mmc4(Box<nintendo::Mmc4>),
    Mmc5(Box<nintendo::Mmc5>),
    Axrom(Box<nintendo::Axrom>),
    Gxrom(Box<nintendo::Gxrom>),
    CpRom(Box<nintendo::CpRom>),
    Mmc1_105(Box<nintendo::Mmc1_105>),
    Mmc1_155(Box<nintendo::Mmc1_155>),
    TxSRom(Box<nintendo::TxSRom>),
    UnRom_94(Box<nintendo::UnRom_94>),
    UnRom_180(Box<nintendo::UnRom_180>),
    CnromProtect(Box<nintendo::CnromProtect>),
    // Konami
    Vrc1(Box<konami::Vrc1>),
    Vrc2_4(Box<konami::Vrc2_4>),
    Vrc3(Box<konami::Vrc3>),
    Vrc6(Box<konami::Vrc6>),
    Vrc7(Box<konami::Vrc7>),
    // Taito
    Tc0190(Box<taito::Tc0190>),
    Tc0690(Box<taito::Tc0690>),
    X1005(Box<taito::X1005>),
    X1017(Box<taito::X1017>),
    // Jaleco
    Jf16(Box<jaleco::Jf16>),
    Jf13(Box<jaleco::Jf13>),
    Jf17_19(Box<jaleco::Jf17_19>),
    Ss88006(Box<jaleco::Ss88006>),
    Jf11_14(Box<jaleco::Jf11_14>),
    // Irem
    G101(Box<irem::G101>),
    H3001(Box<irem::H3001>),
    Lrog017(Box<irem::Lrog017>),
    TamS1(Box<irem::TamS1>),
    // Sunsoft
    Sunsoft3(Box<sunsoft::Sunsoft3>),
    Sunsoft4(Box<sunsoft::Sunsoft4>),
    Fme7(Box<sunsoft::Fme7>),
    Sunsoft89(Box<sunsoft::Sunsoft89>),
    Sunsoft93(Box<sunsoft::Sunsoft93>),
    Sunsoft184(Box<sunsoft::Sunsoft184>),
    // Namco
    Namco163(Box<namco::Namco163>),
    Namco108(Box<namco::Namco108>),
    // Bandai
    Fcg(Box<bandai::Fcg>),
    Bandai74161_7432(Box<bandai::Bandai74161_7432>),
    Karaoke(Box<bandai::Karaoke>),
    // Camerica
    Bf909x(Box<camerica::Bf909x>),
    Bf9096(Box<camerica::Bf9096>),
    // Unlicensed
    ColorDreams(Box<unlicensed::ColorDreams>),
    Rambo1(Box<unlicensed::Rambo1>),
    Mapper15(Box<unlicensed::Mapper15>),
    Mapper35(Box<unlicensed::Mapper35>),
    Mapper40(Box<unlicensed::Mapper40>),
    Mapper42(Box<unlicensed::Mapper42>),
    Mapper43(Box<unlicensed::Mapper43>),
    Mapper50(Box<unlicensed::Mapper50>),
    Mapper57(Box<unlicensed::Mapper57>),
    Mapper58(Box<unlicensed::Mapper58>),
    Mapper60(Box<unlicensed::Mapper60>),
    Mapper61(Box<unlicensed::Mapper61>),
    Mapper62(Box<unlicensed::Mapper62>),
    Mapper79(Box<unlicensed::Mapper79>),
    Mapper83(Box<unlicensed::Mapper83>),
    Mapper91(Box<unlicensed::Mapper91>),
    Mapper103(Box<unlicensed::Mapper103>),
    Mapper106(Box<unlicensed::Mapper106>),
    Mapper107(Box<unlicensed::Mapper107>),
    Mapper112(Box<unlicensed::Mapper112>),
    Mapper116(Box<unlicensed::Mapper116>),
    Mapper117(Box<unlicensed::Mapper117>),
    Mapper120(Box<unlicensed::Mapper120>),
    Mapper170(Box<unlicensed::Mapper170>),
    Mapper174(Box<unlicensed::Mapper174>),
    Mapper200(Box<unlicensed::Mapper200>),
    Mapper202(Box<unlicensed::Mapper202>),
    Mapper203(Box<unlicensed::Mapper203>),
    Mapper204(Box<unlicensed::Mapper204>),
    Mapper212(Box<unlicensed::Mapper212>),
    Mapper213(Box<unlicensed::Mapper213>),
    Mapper214(Box<unlicensed::Mapper214>),
    Mapper216(Box<unlicensed::Mapper216>),
    Mapper221(Box<unlicensed::Mapper221>),
    Mapper222(Box<unlicensed::Mapper222>),
    Mapper225(Box<unlicensed::Mapper225>),
    Mapper226(Box<unlicensed::Mapper226>),
    Mapper227(Box<unlicensed::Mapper227>),
    Mapper229(Box<unlicensed::Mapper229>),
    Mapper230(Box<unlicensed::Mapper230>),
    Mapper231(Box<unlicensed::Mapper231>),
    Mapper233(Box<unlicensed::Mapper233>),
    Mapper234(Box<unlicensed::Mapper234>),
    Mapper240(Box<unlicensed::Mapper240>),
    Mapper241(Box<unlicensed::Mapper241>),
    Mapper242(Box<unlicensed::Mapper242>),
    Mapper244(Box<unlicensed::Mapper244>),
    Mapper246(Box<unlicensed::Mapper246>),
    Mapper253(Box<unlicensed::Mapper253>),
    OekaKids(Box<unlicensed::OekaKids>),
    Null,
}

impl Mapper {
    #[allow(clippy::match_same_arms)]
    pub fn from_ines(
        id: u8,
        mirroring: u8,
        prg_data: &[u8],
        chr_data: &[u8],
        #[allow(dead_code)] chr_ram: bool,
    ) -> Self {
        match id {
            0 => Self::Nrom(Box::new(nintendo::Nrom::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            1 => Self::Mmc1(Box::new(nintendo::Mmc1::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            2 => Self::UxRom(Box::new(nintendo::UxRom::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            3 => Self::Cnrom(Box::new(nintendo::Cnrom::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            4 => Self::Mmc3(Box::new(nintendo::Mmc3::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            5 => Self::Mmc5(Box::new(nintendo::Mmc5::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            7 => Self::Axrom(Box::new(nintendo::Axrom::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            9 => Self::Mmc2(Box::new(nintendo::Mmc2::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            10 => Self::Mmc4(Box::new(nintendo::Mmc4::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            11 | 144 => Self::ColorDreams(Box::new(unlicensed::ColorDreams::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            13 => Self::CpRom(Box::new(nintendo::CpRom::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            15 => Self::Mapper15(Box::new(unlicensed::Mapper15::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            16 | 153 | 157 | 159 => Self::Fcg(Box::new(bandai::Fcg::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            18 => Self::Ss88006(Box::new(jaleco::Ss88006::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            19 | 210 => Self::Namco163(Box::new(namco::Namco163::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            21 | 22 | 23 | 25 => Self::Vrc2_4(Box::new(konami::Vrc2_4::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            24 | 26 => Self::Vrc6(Box::new(konami::Vrc6::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            32 => Self::G101(Box::new(irem::G101::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            33 => Self::Tc0190(Box::new(taito::Tc0190::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            35 => Self::Mapper35(Box::new(unlicensed::Mapper35::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            40 => Self::Mapper40(Box::new(unlicensed::Mapper40::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            42 => Self::Mapper42(Box::new(unlicensed::Mapper42::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            43 => Self::Mapper43(Box::new(unlicensed::Mapper43::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            48 => Self::Tc0690(Box::new(taito::Tc0690::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            50 => Self::Mapper50(Box::new(unlicensed::Mapper50::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            57 => Self::Mapper57(Box::new(unlicensed::Mapper57::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            58 => Self::Mapper58(Box::new(unlicensed::Mapper58::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            60 => Self::Mapper60(Box::new(unlicensed::Mapper60::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            61 => Self::Mapper61(Box::new(unlicensed::Mapper61::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            62 => Self::Mapper62(Box::new(unlicensed::Mapper62::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            64 | 158 => Self::Rambo1(Box::new(unlicensed::Rambo1::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            65 => Self::H3001(Box::new(irem::H3001::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            66 => Self::Gxrom(Box::new(nintendo::Gxrom::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            67 => Self::Sunsoft3(Box::new(sunsoft::Sunsoft3::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            68 => Self::Sunsoft4(Box::new(sunsoft::Sunsoft4::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            69 => Self::Fme7(Box::new(sunsoft::Fme7::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            70 | 152 => Self::Bandai74161_7432(Box::new(bandai::Bandai74161_7432::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            71 => Self::Bf909x(Box::new(camerica::Bf909x::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            72 | 92 => Self::Jf17_19(Box::new(jaleco::Jf17_19::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            73 => Self::Vrc3(Box::new(konami::Vrc3::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            74 | 119 | 191 | 194 | 195 | 192 => Self::TxSRom(Box::new(nintendo::TxSRom::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            75 => Self::Vrc1(Box::new(konami::Vrc1::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            76 => Self::Namco108(Box::new(namco::Namco108::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            77 => Self::Lrog017(Box::new(irem::Lrog017::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            78 => Self::Jf16(Box::new(jaleco::Jf16::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            79 | 113 | 146 => Self::Mapper79(Box::new(unlicensed::Mapper79::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            80 | 207 => Self::X1005(Box::new(taito::X1005::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            82 => Self::X1017(Box::new(taito::X1017::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            83 => Self::Mapper83(Box::new(unlicensed::Mapper83::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            85 => Self::Vrc7(Box::new(konami::Vrc7::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            86 => Self::Jf13(Box::new(jaleco::Jf13::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            88 => Self::Namco108(Box::new(namco::Namco108::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            89 => Self::Sunsoft89(Box::new(sunsoft::Sunsoft89::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            91 => Self::Mapper91(Box::new(unlicensed::Mapper91::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            93 => Self::Sunsoft93(Box::new(sunsoft::Sunsoft93::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            94 => Self::UnRom_94(Box::new(nintendo::UnRom_94::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            95 => Self::Namco108(Box::new(namco::Namco108::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            96 => Self::OekaKids(Box::new(unlicensed::OekaKids::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            97 => Self::TamS1(Box::new(irem::TamS1::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            103 => Self::Mapper103(Box::new(unlicensed::Mapper103::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            105 => Self::Mmc1_105(Box::new(nintendo::Mmc1_105::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            106 => Self::Mapper106(Box::new(unlicensed::Mapper106::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            107 => Self::Mapper107(Box::new(unlicensed::Mapper107::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            112 => Self::Mapper112(Box::new(unlicensed::Mapper112::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            116 => Self::Mapper116(Box::new(unlicensed::Mapper116::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            117 => Self::Mapper117(Box::new(unlicensed::Mapper117::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            118 => Self::TxSRom(Box::new(nintendo::TxSRom::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            120 => Self::Mapper120(Box::new(unlicensed::Mapper120::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            140 => Self::Jf11_14(Box::new(jaleco::Jf11_14::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            154 => Self::Namco108(Box::new(namco::Namco108::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            155 => Self::Mmc1_155(Box::new(nintendo::Mmc1_155::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            170 => Self::Mapper170(Box::new(unlicensed::Mapper170::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            174 => Self::Mapper174(Box::new(unlicensed::Mapper174::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            180 => Self::UnRom_180(Box::new(nintendo::UnRom_180::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            184 => Self::Sunsoft184(Box::new(sunsoft::Sunsoft184::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            185 => Self::CnromProtect(Box::new(nintendo::CnromProtect::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            188 => Self::Karaoke(Box::new(bandai::Karaoke::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            200 => Self::Mapper200(Box::new(unlicensed::Mapper200::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            202 => Self::Mapper202(Box::new(unlicensed::Mapper202::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            203 => Self::Mapper203(Box::new(unlicensed::Mapper203::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            204 => Self::Mapper204(Box::new(unlicensed::Mapper204::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            206 => Self::Namco108(Box::new(namco::Namco108::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            212 => Self::Mapper212(Box::new(unlicensed::Mapper212::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            213 => Self::Mapper213(Box::new(unlicensed::Mapper213::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            214 => Self::Mapper214(Box::new(unlicensed::Mapper214::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            216 => Self::Mapper216(Box::new(unlicensed::Mapper216::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            221 => Self::Mapper221(Box::new(unlicensed::Mapper221::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            222 => Self::Mapper222(Box::new(unlicensed::Mapper222::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            225 => Self::Mapper225(Box::new(unlicensed::Mapper225::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            226 => Self::Mapper226(Box::new(unlicensed::Mapper226::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            227 => Self::Mapper227(Box::new(unlicensed::Mapper227::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            229 => Self::Mapper229(Box::new(unlicensed::Mapper229::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            230 => Self::Mapper230(Box::new(unlicensed::Mapper230::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            231 => Self::Mapper231(Box::new(unlicensed::Mapper231::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            232 => Self::Bf9096(Box::new(camerica::Bf9096::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            233 => Self::Mapper233(Box::new(unlicensed::Mapper233::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            234 => Self::Mapper234(Box::new(unlicensed::Mapper234::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            240 => Self::Mapper240(Box::new(unlicensed::Mapper240::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            241 => Self::Mapper241(Box::new(unlicensed::Mapper241::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            242 => Self::Mapper242(Box::new(unlicensed::Mapper242::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            244 => Self::Mapper244(Box::new(unlicensed::Mapper244::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            246 => Self::Mapper246(Box::new(unlicensed::Mapper246::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            253 => Self::Mapper253(Box::new(unlicensed::Mapper253::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
            // Fallback: NROM for unknown mappers
            _ => Self::Nrom(Box::new(nintendo::Nrom::new(
                prg_data, chr_data, chr_ram, mirroring,
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatch macros
// ---------------------------------------------------------------------------

macro_rules! dispatch_mut {
    // No-argument methods (returns bool or u8)
    ($self:expr, $method:ident; Null => $null_val:expr) => {
        match $self {
            Self::Nrom(m) => m.$method(),
            Self::UxRom(m) => m.$method(),
            Self::Cnrom(m) => m.$method(),
            Self::Mmc1(m) => m.$method(),
            Self::Mmc2(m) => m.$method(),
            Self::Mmc3(m) => m.$method(),
            Self::Mmc4(m) => m.$method(),
            Self::Mmc5(m) => m.$method(),
            Self::Axrom(m) => m.$method(),
            Self::Gxrom(m) => m.$method(),
            Self::CpRom(m) => m.$method(),
            Self::Mmc1_105(m) => m.$method(),
            Self::Mmc1_155(m) => m.$method(),
            Self::TxSRom(m) => m.$method(),
            Self::UnRom_94(m) => m.$method(),
            Self::UnRom_180(m) => m.$method(),
            Self::CnromProtect(m) => m.$method(),
            Self::Vrc1(m) => m.$method(),
            Self::Vrc2_4(m) => m.$method(),
            Self::Vrc3(m) => m.$method(),
            Self::Vrc6(m) => m.$method(),
            Self::Vrc7(m) => m.$method(),
            Self::Tc0190(m) => m.$method(),
            Self::Tc0690(m) => m.$method(),
            Self::X1005(m) => m.$method(),
            Self::X1017(m) => m.$method(),
            Self::Jf16(m) => m.$method(),
            Self::Jf13(m) => m.$method(),
            Self::Jf17_19(m) => m.$method(),
            Self::Ss88006(m) => m.$method(),
            Self::Jf11_14(m) => m.$method(),
            Self::G101(m) => m.$method(),
            Self::H3001(m) => m.$method(),
            Self::Lrog017(m) => m.$method(),
            Self::TamS1(m) => m.$method(),
            Self::Sunsoft3(m) => m.$method(),
            Self::Sunsoft4(m) => m.$method(),
            Self::Fme7(m) => m.$method(),
            Self::Sunsoft89(m) => m.$method(),
            Self::Sunsoft93(m) => m.$method(),
            Self::Sunsoft184(m) => m.$method(),
            Self::Namco163(m) => m.$method(),
            Self::Namco108(m) => m.$method(),
            Self::Fcg(m) => m.$method(),
            Self::Bandai74161_7432(m) => m.$method(),
            Self::Karaoke(m) => m.$method(),
            Self::Bf909x(m) => m.$method(),
            Self::Bf9096(m) => m.$method(),
            Self::ColorDreams(m) => m.$method(),
            Self::Rambo1(m) => m.$method(),
            Self::Mapper15(m) => m.$method(),
            Self::Mapper35(m) => m.$method(),
            Self::Mapper40(m) => m.$method(),
            Self::Mapper42(m) => m.$method(),
            Self::Mapper43(m) => m.$method(),
            Self::Mapper50(m) => m.$method(),
            Self::Mapper57(m) => m.$method(),
            Self::Mapper58(m) => m.$method(),
            Self::Mapper60(m) => m.$method(),
            Self::Mapper61(m) => m.$method(),
            Self::Mapper62(m) => m.$method(),
            Self::Mapper79(m) => m.$method(),
            Self::Mapper83(m) => m.$method(),
            Self::Mapper91(m) => m.$method(),
            Self::Mapper103(m) => m.$method(),
            Self::Mapper106(m) => m.$method(),
            Self::Mapper107(m) => m.$method(),
            Self::Mapper112(m) => m.$method(),
            Self::Mapper116(m) => m.$method(),
            Self::Mapper117(m) => m.$method(),
            Self::Mapper120(m) => m.$method(),
            Self::Mapper170(m) => m.$method(),
            Self::Mapper174(m) => m.$method(),
            Self::Mapper200(m) => m.$method(),
            Self::Mapper202(m) => m.$method(),
            Self::Mapper203(m) => m.$method(),
            Self::Mapper204(m) => m.$method(),
            Self::Mapper212(m) => m.$method(),
            Self::Mapper213(m) => m.$method(),
            Self::Mapper214(m) => m.$method(),
            Self::Mapper216(m) => m.$method(),
            Self::Mapper221(m) => m.$method(),
            Self::Mapper222(m) => m.$method(),
            Self::Mapper225(m) => m.$method(),
            Self::Mapper226(m) => m.$method(),
            Self::Mapper227(m) => m.$method(),
            Self::Mapper229(m) => m.$method(),
            Self::Mapper230(m) => m.$method(),
            Self::Mapper231(m) => m.$method(),
            Self::Mapper233(m) => m.$method(),
            Self::Mapper234(m) => m.$method(),
            Self::Mapper240(m) => m.$method(),
            Self::Mapper241(m) => m.$method(),
            Self::Mapper242(m) => m.$method(),
            Self::Mapper244(m) => m.$method(),
            Self::Mapper246(m) => m.$method(),
            Self::Mapper253(m) => m.$method(),
            Self::OekaKids(m) => m.$method(),
            Self::Null => $null_val,
        }
    };
    ($self:expr, $method:ident, $arg1:expr; Null => $null_val:expr) => {
        match $self {
            Self::Nrom(m) => m.$method($arg1),
            Self::UxRom(m) => m.$method($arg1),
            Self::Cnrom(m) => m.$method($arg1),
            Self::Mmc1(m) => m.$method($arg1),
            Self::Mmc2(m) => m.$method($arg1),
            Self::Mmc3(m) => m.$method($arg1),
            Self::Mmc4(m) => m.$method($arg1),
            Self::Mmc5(m) => m.$method($arg1),
            Self::Axrom(m) => m.$method($arg1),
            Self::Gxrom(m) => m.$method($arg1),
            Self::CpRom(m) => m.$method($arg1),
            Self::Mmc1_105(m) => m.$method($arg1),
            Self::Mmc1_155(m) => m.$method($arg1),
            Self::TxSRom(m) => m.$method($arg1),
            Self::UnRom_94(m) => m.$method($arg1),
            Self::UnRom_180(m) => m.$method($arg1),
            Self::CnromProtect(m) => m.$method($arg1),
            Self::Vrc1(m) => m.$method($arg1),
            Self::Vrc2_4(m) => m.$method($arg1),
            Self::Vrc3(m) => m.$method($arg1),
            Self::Vrc6(m) => m.$method($arg1),
            Self::Vrc7(m) => m.$method($arg1),
            Self::Tc0190(m) => m.$method($arg1),
            Self::Tc0690(m) => m.$method($arg1),
            Self::X1005(m) => m.$method($arg1),
            Self::X1017(m) => m.$method($arg1),
            Self::Jf16(m) => m.$method($arg1),
            Self::Jf13(m) => m.$method($arg1),
            Self::Jf17_19(m) => m.$method($arg1),
            Self::Ss88006(m) => m.$method($arg1),
            Self::Jf11_14(m) => m.$method($arg1),
            Self::G101(m) => m.$method($arg1),
            Self::H3001(m) => m.$method($arg1),
            Self::Lrog017(m) => m.$method($arg1),
            Self::TamS1(m) => m.$method($arg1),
            Self::Sunsoft3(m) => m.$method($arg1),
            Self::Sunsoft4(m) => m.$method($arg1),
            Self::Fme7(m) => m.$method($arg1),
            Self::Sunsoft89(m) => m.$method($arg1),
            Self::Sunsoft93(m) => m.$method($arg1),
            Self::Sunsoft184(m) => m.$method($arg1),
            Self::Namco163(m) => m.$method($arg1),
            Self::Namco108(m) => m.$method($arg1),
            Self::Fcg(m) => m.$method($arg1),
            Self::Bandai74161_7432(m) => m.$method($arg1),
            Self::Karaoke(m) => m.$method($arg1),
            Self::Bf909x(m) => m.$method($arg1),
            Self::Bf9096(m) => m.$method($arg1),
            Self::ColorDreams(m) => m.$method($arg1),
            Self::Rambo1(m) => m.$method($arg1),
            Self::Mapper15(m) => m.$method($arg1),
            Self::Mapper35(m) => m.$method($arg1),
            Self::Mapper40(m) => m.$method($arg1),
            Self::Mapper42(m) => m.$method($arg1),
            Self::Mapper43(m) => m.$method($arg1),
            Self::Mapper50(m) => m.$method($arg1),
            Self::Mapper57(m) => m.$method($arg1),
            Self::Mapper58(m) => m.$method($arg1),
            Self::Mapper60(m) => m.$method($arg1),
            Self::Mapper61(m) => m.$method($arg1),
            Self::Mapper62(m) => m.$method($arg1),
            Self::Mapper79(m) => m.$method($arg1),
            Self::Mapper83(m) => m.$method($arg1),
            Self::Mapper91(m) => m.$method($arg1),
            Self::Mapper103(m) => m.$method($arg1),
            Self::Mapper106(m) => m.$method($arg1),
            Self::Mapper107(m) => m.$method($arg1),
            Self::Mapper112(m) => m.$method($arg1),
            Self::Mapper116(m) => m.$method($arg1),
            Self::Mapper117(m) => m.$method($arg1),
            Self::Mapper120(m) => m.$method($arg1),
            Self::Mapper170(m) => m.$method($arg1),
            Self::Mapper174(m) => m.$method($arg1),
            Self::Mapper200(m) => m.$method($arg1),
            Self::Mapper202(m) => m.$method($arg1),
            Self::Mapper203(m) => m.$method($arg1),
            Self::Mapper204(m) => m.$method($arg1),
            Self::Mapper212(m) => m.$method($arg1),
            Self::Mapper213(m) => m.$method($arg1),
            Self::Mapper214(m) => m.$method($arg1),
            Self::Mapper216(m) => m.$method($arg1),
            Self::Mapper221(m) => m.$method($arg1),
            Self::Mapper222(m) => m.$method($arg1),
            Self::Mapper225(m) => m.$method($arg1),
            Self::Mapper226(m) => m.$method($arg1),
            Self::Mapper227(m) => m.$method($arg1),
            Self::Mapper229(m) => m.$method($arg1),
            Self::Mapper230(m) => m.$method($arg1),
            Self::Mapper231(m) => m.$method($arg1),
            Self::Mapper233(m) => m.$method($arg1),
            Self::Mapper234(m) => m.$method($arg1),
            Self::Mapper240(m) => m.$method($arg1),
            Self::Mapper241(m) => m.$method($arg1),
            Self::Mapper242(m) => m.$method($arg1),
            Self::Mapper244(m) => m.$method($arg1),
            Self::Mapper246(m) => m.$method($arg1),
            Self::Mapper253(m) => m.$method($arg1),
            Self::OekaKids(m) => m.$method($arg1),
            Self::Null => $null_val,
        }
    };
    ($self:expr, $method:ident, $arg1:expr, $arg2:expr; Null => $null_val:expr) => {
        match $self {
            Self::Nrom(m) => m.$method($arg1, $arg2),
            Self::UxRom(m) => m.$method($arg1, $arg2),
            Self::Cnrom(m) => m.$method($arg1, $arg2),
            Self::Mmc1(m) => m.$method($arg1, $arg2),
            Self::Mmc2(m) => m.$method($arg1, $arg2),
            Self::Mmc3(m) => m.$method($arg1, $arg2),
            Self::Mmc4(m) => m.$method($arg1, $arg2),
            Self::Mmc5(m) => m.$method($arg1, $arg2),
            Self::Axrom(m) => m.$method($arg1, $arg2),
            Self::Gxrom(m) => m.$method($arg1, $arg2),
            Self::CpRom(m) => m.$method($arg1, $arg2),
            Self::Mmc1_105(m) => m.$method($arg1, $arg2),
            Self::Mmc1_155(m) => m.$method($arg1, $arg2),
            Self::TxSRom(m) => m.$method($arg1, $arg2),
            Self::UnRom_94(m) => m.$method($arg1, $arg2),
            Self::UnRom_180(m) => m.$method($arg1, $arg2),
            Self::CnromProtect(m) => m.$method($arg1, $arg2),
            Self::Vrc1(m) => m.$method($arg1, $arg2),
            Self::Vrc2_4(m) => m.$method($arg1, $arg2),
            Self::Vrc3(m) => m.$method($arg1, $arg2),
            Self::Vrc6(m) => m.$method($arg1, $arg2),
            Self::Vrc7(m) => m.$method($arg1, $arg2),
            Self::Tc0190(m) => m.$method($arg1, $arg2),
            Self::Tc0690(m) => m.$method($arg1, $arg2),
            Self::X1005(m) => m.$method($arg1, $arg2),
            Self::X1017(m) => m.$method($arg1, $arg2),
            Self::Jf16(m) => m.$method($arg1, $arg2),
            Self::Jf13(m) => m.$method($arg1, $arg2),
            Self::Jf17_19(m) => m.$method($arg1, $arg2),
            Self::Ss88006(m) => m.$method($arg1, $arg2),
            Self::Jf11_14(m) => m.$method($arg1, $arg2),
            Self::G101(m) => m.$method($arg1, $arg2),
            Self::H3001(m) => m.$method($arg1, $arg2),
            Self::Lrog017(m) => m.$method($arg1, $arg2),
            Self::TamS1(m) => m.$method($arg1, $arg2),
            Self::Sunsoft3(m) => m.$method($arg1, $arg2),
            Self::Sunsoft4(m) => m.$method($arg1, $arg2),
            Self::Fme7(m) => m.$method($arg1, $arg2),
            Self::Sunsoft89(m) => m.$method($arg1, $arg2),
            Self::Sunsoft93(m) => m.$method($arg1, $arg2),
            Self::Sunsoft184(m) => m.$method($arg1, $arg2),
            Self::Namco163(m) => m.$method($arg1, $arg2),
            Self::Namco108(m) => m.$method($arg1, $arg2),
            Self::Fcg(m) => m.$method($arg1, $arg2),
            Self::Bandai74161_7432(m) => m.$method($arg1, $arg2),
            Self::Karaoke(m) => m.$method($arg1, $arg2),
            Self::Bf909x(m) => m.$method($arg1, $arg2),
            Self::Bf9096(m) => m.$method($arg1, $arg2),
            Self::ColorDreams(m) => m.$method($arg1, $arg2),
            Self::Rambo1(m) => m.$method($arg1, $arg2),
            Self::Mapper15(m) => m.$method($arg1, $arg2),
            Self::Mapper35(m) => m.$method($arg1, $arg2),
            Self::Mapper40(m) => m.$method($arg1, $arg2),
            Self::Mapper42(m) => m.$method($arg1, $arg2),
            Self::Mapper43(m) => m.$method($arg1, $arg2),
            Self::Mapper50(m) => m.$method($arg1, $arg2),
            Self::Mapper57(m) => m.$method($arg1, $arg2),
            Self::Mapper58(m) => m.$method($arg1, $arg2),
            Self::Mapper60(m) => m.$method($arg1, $arg2),
            Self::Mapper61(m) => m.$method($arg1, $arg2),
            Self::Mapper62(m) => m.$method($arg1, $arg2),
            Self::Mapper79(m) => m.$method($arg1, $arg2),
            Self::Mapper83(m) => m.$method($arg1, $arg2),
            Self::Mapper91(m) => m.$method($arg1, $arg2),
            Self::Mapper103(m) => m.$method($arg1, $arg2),
            Self::Mapper106(m) => m.$method($arg1, $arg2),
            Self::Mapper107(m) => m.$method($arg1, $arg2),
            Self::Mapper112(m) => m.$method($arg1, $arg2),
            Self::Mapper116(m) => m.$method($arg1, $arg2),
            Self::Mapper117(m) => m.$method($arg1, $arg2),
            Self::Mapper120(m) => m.$method($arg1, $arg2),
            Self::Mapper170(m) => m.$method($arg1, $arg2),
            Self::Mapper174(m) => m.$method($arg1, $arg2),
            Self::Mapper200(m) => m.$method($arg1, $arg2),
            Self::Mapper202(m) => m.$method($arg1, $arg2),
            Self::Mapper203(m) => m.$method($arg1, $arg2),
            Self::Mapper204(m) => m.$method($arg1, $arg2),
            Self::Mapper212(m) => m.$method($arg1, $arg2),
            Self::Mapper213(m) => m.$method($arg1, $arg2),
            Self::Mapper214(m) => m.$method($arg1, $arg2),
            Self::Mapper216(m) => m.$method($arg1, $arg2),
            Self::Mapper221(m) => m.$method($arg1, $arg2),
            Self::Mapper222(m) => m.$method($arg1, $arg2),
            Self::Mapper225(m) => m.$method($arg1, $arg2),
            Self::Mapper226(m) => m.$method($arg1, $arg2),
            Self::Mapper227(m) => m.$method($arg1, $arg2),
            Self::Mapper229(m) => m.$method($arg1, $arg2),
            Self::Mapper230(m) => m.$method($arg1, $arg2),
            Self::Mapper231(m) => m.$method($arg1, $arg2),
            Self::Mapper233(m) => m.$method($arg1, $arg2),
            Self::Mapper234(m) => m.$method($arg1, $arg2),
            Self::Mapper240(m) => m.$method($arg1, $arg2),
            Self::Mapper241(m) => m.$method($arg1, $arg2),
            Self::Mapper242(m) => m.$method($arg1, $arg2),
            Self::Mapper244(m) => m.$method($arg1, $arg2),
            Self::Mapper246(m) => m.$method($arg1, $arg2),
            Self::Mapper253(m) => m.$method($arg1, $arg2),
            Self::OekaKids(m) => m.$method($arg1, $arg2),
            Self::Null => $null_val,
        }
    };
}

#[rustfmt::skip]
impl Mapper {
    #[inline(always)]
    pub fn cpu_read(&mut self, addr: u16) -> u8 { dispatch_mut!(self, cpu_read, addr; Null => 0) }
    #[inline(always)]
    pub fn cpu_write(&mut self, addr: u16, val: u8) { dispatch_mut!(self, cpu_write, addr, val; Null => {}) }
    #[inline(always)]
    pub fn ppu_read(&mut self, addr: u16) -> u8 { dispatch_mut!(self, ppu_read, addr; Null => 0) }
    #[inline(always)]
    pub fn ppu_write(&mut self, addr: u16, val: u8) { dispatch_mut!(self, ppu_write, addr, val; Null => {}) }
    pub fn mirroring(&self) -> u8 { dispatch_mut!(self, mirroring; Null => 0) }
    pub fn irq_pending(&self) -> bool { dispatch_mut!(self, irq_pending; Null => false) }
    pub fn ack_irq(&mut self) { dispatch_mut!(self, ack_irq; Null => {}) }
    pub fn clock_scanline(&mut self) { dispatch_mut!(self, clock_scanline; Null => {}) }
    pub fn has_chr_ram(&self) -> bool { dispatch_mut!(self, has_chr_ram; Null => true) }

    pub fn notify_scanline(&mut self, scanline: u16) {
        match self {
            Self::Mmc2(m) => m.notify_scanline(scanline),
            Self::Mmc4(m) => m.notify_scanline(scanline),
            Self::Mmc5(m) => m.notify_scanline(scanline),
            Self::Mmc1_105(m) => m.notify_scanline(scanline),
            Self::Mmc1_155(m) => m.notify_scanline(scanline),
            Self::Vrc6(m) => m.notify_scanline(scanline),
            Self::Vrc7(m) => m.notify_scanline(scanline),
            Self::X1005(m) => m.notify_scanline(scanline),
            Self::X1017(m) => m.notify_scanline(scanline),
            Self::OekaKids(m) => m.notify_scanline(scanline),
            Self::Fcg(m) => m.notify_scanline(scanline),
            Self::Ss88006(m) => m.notify_scanline(scanline),
            Self::Namco163(m) => m.notify_scanline(scanline),
            Self::Fme7(m) => m.notify_scanline(scanline),
            Self::Rambo1(m) => m.notify_scanline(scanline),
            Self::H3001(m) => m.notify_scanline(scanline),
            Self::TamS1(m) => m.notify_scanline(scanline),
            Self::TxSRom(m) => m.notify_scanline(scanline),
            Self::Vrc2_4(m) => m.notify_scanline(scanline),
            _ => {}
        }
    }

    pub fn nt_mapping(&self) -> u8 {
        match self {
            Self::Mmc2(m) => m.nt_mapping(),
            Self::Mmc4(m) => m.nt_mapping(),
            Self::Mmc5(m) => m.nt_mapping(),
            _ => 0xFF,
        }
    }

    pub fn read_nt_ext(&mut self, addr: u16, nt_source: u8) -> u8 {
        match self {
            Self::Mmc2(m) => m.read_nt_ext(addr, nt_source),
            Self::Mmc4(m) => m.read_nt_ext(addr, nt_source),
            Self::Mmc5(m) => m.read_nt_ext(addr, nt_source),
            _ => 0,
        }
    }

    pub fn write_nt_ext(&mut self, addr: u16, nt_source: u8, val: u8) {
        match self {
            Self::Mmc2(m) => m.write_nt_ext(addr, nt_source, val),
            Self::Mmc4(m) => m.write_nt_ext(addr, nt_source, val),
            Self::Mmc5(m) => m.write_nt_ext(addr, nt_source, val),
            _ => {}
        }
    }

    pub fn set_chr_fetch_bg(&mut self) {
        match self {
            Self::Mmc2(m) => m.set_chr_fetch_bg(),
            Self::Mmc4(m) => m.set_chr_fetch_bg(),
            Self::Mmc5(m) => m.set_chr_fetch_bg(),
            _ => {}
        }
    }

    pub fn set_chr_fetch_sprite(&mut self) {
        match self {
            Self::Mmc2(m) => m.set_chr_fetch_sprite(),
            Self::Mmc4(m) => m.set_chr_fetch_sprite(),
            Self::Mmc5(m) => m.set_chr_fetch_sprite(),
            _ => {}
        }
    }

    pub fn set_extended_chr_bank(&mut self, bank: u8) {
        match self {
            Self::Mmc2(m) => m.set_extended_chr_bank(bank),
            Self::Mmc4(m) => m.set_extended_chr_bank(bank),
            Self::Mmc5(m) => m.set_extended_chr_bank(bank),
            _ => {}
        }
    }

    pub fn get_extended_chr_bank(&self) -> u8 {
        match self {
            Self::Mmc2(m) => m.get_extended_chr_bank(),
            Self::Mmc4(m) => m.get_extended_chr_bank(),
            Self::Mmc5(m) => m.get_extended_chr_bank(),
            _ => 0,
        }
    }

    pub fn get_ex_ram_mode(&self) -> u8 {
        match self {
            Self::Mmc2(m) => m.get_ex_ram_mode(),
            Self::Mmc4(m) => m.get_ex_ram_mode(),
            Self::Mmc5(m) => m.get_ex_ram_mode(),
            _ => 0,
        }
    }

    pub fn get_fill_tile(&self) -> u8 {
        match self {
            Self::Mmc2(m) => m.get_fill_tile(),
            Self::Mmc4(m) => m.get_fill_tile(),
            Self::Mmc5(m) => m.get_fill_tile(),
            _ => 0,
        }
    }

    pub fn get_fill_attr(&self) -> u8 {
        match self {
            Self::Mmc2(m) => m.get_fill_attr(),
            Self::Mmc4(m) => m.get_fill_attr(),
            Self::Mmc5(m) => m.get_fill_attr(),
            _ => 0,
        }
    }

    pub fn read_ex_ram_byte(&mut self, offset: u16) -> u8 {
        match self {
            Self::Mmc2(m) => m.read_ex_ram_byte(offset),
            Self::Mmc4(m) => m.read_ex_ram_byte(offset),
            Self::Mmc5(m) => m.read_ex_ram_byte(offset),
            _ => 0,
        }
    }
}
