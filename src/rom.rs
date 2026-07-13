pub struct Rom {
    pub prg: [u8; 0x8000],
    pub chr: [u8; 0x2000],
    pub prg_size: usize,
    pub chr_size: usize,
    pub mapper: u8,
    pub mirroring: u8,
    pub has_chr_ram: bool,
}

impl Rom {
    pub fn new(data: &[u8]) -> Option<Self> {
        if data.len() < 16 || data[0..4] != [0x4E, 0x45, 0x53, 0x1A] {
            return None;
        }
        let prg_16kb = data[4] as usize;
        let chr_8kb = data[5] as usize;
        let flags6 = data[6];
        let flags7 = data[7];
        let mapper = (flags7 & 0xF0) | (flags6 >> 4);
        let mirroring = flags6 & 0x01;
        let has_chr_ram = chr_8kb == 0;

        let prg_size = prg_16kb * 0x4000;
        let chr_size = if chr_8kb == 0 {
            0x2000
        } else {
            chr_8kb * 0x2000
        };

        let header_size = if flags6 & 0x04 != 0 { 16 + 512 } else { 16 };
        let mut prg = [0u8; 0x8000];
        let mut chr = [0u8; 0x2000];

        let prg_src = &data[header_size..header_size + prg_size];
        if prg_size <= 0x8000 {
            prg[..prg_size].copy_from_slice(prg_src);
            if prg_size == 0x4000 {
                prg[0x4000..0x8000].copy_from_slice(prg_src);
            }
        }

        let chr_start = header_size + prg_size;
        if chr_size > 0 {
            let chr_src = &data[chr_start..chr_start + chr_size.min(0x2000)];
            chr[..chr_src.len()].copy_from_slice(chr_src);
        }

        Some(Self {
            prg,
            chr,
            prg_size,
            chr_size,
            mapper,
            mirroring,
            has_chr_ram,
        })
    }

    pub fn read_prg(&self, addr: u16) -> u8 {
        let idx = (addr & 0x7FFF) as usize;
        if self.prg_size <= 0x4000 && idx >= 0x4000 {
            self.prg[idx % 0x4000]
        } else {
            self.prg[idx.min(0x7FFF)]
        }
    }
}
