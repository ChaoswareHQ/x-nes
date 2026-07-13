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
        let header_size = if flags6 & 0x04 != 0 { 16 + 512 } else { 16 };

        let mut prg = [0u8; 0x8000];
        let mut chr = [0u8; 0x2000];

        let data_start = header_size;
        let data_end = data.len();
        let prg_end = (data_start + prg_size).min(data_end);
        let prg_real = prg_end - data_start;

        if prg_real > 0 {
            let prg_src = if prg_real > 0x8000 {
                &data[prg_end - 0x8000..prg_end]
            } else {
                &data[data_start..prg_end]
            };
            prg[..prg_src.len()].copy_from_slice(prg_src);
            if prg_real == 0x4000 {
                prg[0x4000..0x8000].copy_from_slice(prg_src);
            }
        }

        if !has_chr_ram {
            let chr_size = chr_8kb * 0x2000;
            let chr_start = data_start + prg_size;
            let chr_end = (chr_start + chr_size).min(data_end);
            let chr_real = chr_end - chr_start;
            if chr_real > 0 {
                let chr_src = &data[chr_start..chr_end];
                let copy_len = chr_src.len().min(0x2000);
                chr[..copy_len].copy_from_slice(&chr_src[..copy_len]);
            }
        }

        Some(Self {
            prg,
            chr,
            prg_size: prg_real.min(0x8000),
            chr_size: if has_chr_ram { 0 } else { chr_8kb * 0x2000 },
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_header(prg_banks: u8, chr_banks: u8, flags6: u8, flags7: u8) -> Vec<u8> {
        let mut h = vec![0x4E, 0x45, 0x53, 0x1A, prg_banks, chr_banks, flags6, flags7];
        h.resize(16, 0);
        h
    }

    #[test]
    fn invalid_header_rejected() {
        assert!(Rom::new(b"").is_none());
        assert!(Rom::new(b"NOPE").is_none());
        assert!(Rom::new(&[0; 16]).is_none());
        let mut valid = vec![0x4E, 0x45, 0x53, 0x1A, 1, 0, 0, 0];
        valid.resize(16, 0);
        valid.extend(&[0xAB; 0x4000]);
        assert!(Rom::new(&valid).is_some());
    }

    #[test]
    fn nrom_16kb_prg_no_chr() {
        let prg = vec![0xABu8; 0x4000];
        let mut data = fake_header(1, 0, 0x00, 0x00);
        data.extend(&prg);
        let rom = Rom::new(&data).unwrap();

        assert_eq!(rom.mapper, 0);
        assert_eq!(rom.mirroring, 0);
        assert!(rom.has_chr_ram);
        assert_eq!(rom.prg[0], 0xAB);
        assert_eq!(rom.prg[0x3FFF], 0xAB);
        assert_eq!(rom.prg[0x4000], 0xAB);
        assert_eq!(rom.prg[0x7FFF], 0xAB);
    }

    #[test]
    fn nrom_16kb_prg_with_chr() {
        let prg = vec![0xCDu8; 0x4000];
        let chr = vec![0xEFu8; 0x2000];
        let mut data = fake_header(1, 1, 0x00, 0x00);
        data.extend(&prg);
        data.extend(&chr);
        let rom = Rom::new(&data).unwrap();

        assert_eq!(rom.mapper, 0);
        assert!(!rom.has_chr_ram);
        assert_eq!(rom.chr[0], 0xEF);
        assert_eq!(rom.chr[0x1FFF], 0xEF);
    }

    #[test]
    fn nrom_32kb_prg() {
        let prg = vec![0x42u8; 0x8000];
        let mut data = fake_header(2, 0, 0x00, 0x00);
        data.extend(&prg);
        let rom = Rom::new(&data).unwrap();

        assert_eq!(rom.prg[0], 0x42);
        assert_eq!(rom.prg[0x7FFF], 0x42);
        assert!(rom.has_chr_ram);
    }

    #[test]
    fn mapper_detected() {
        let data = fake_header(1, 0, 0x50, 0x10);
        let rom = Rom::new(&data).unwrap();
        assert_eq!(rom.mapper, 0x15);
    }

    #[test]
    fn mirroring_detected() {
        let h = fake_header(1, 0, 0x01, 0x00);
        assert_eq!(Rom::new(&h).unwrap().mirroring, 1);
        let v = fake_header(1, 0, 0x00, 0x00);
        assert_eq!(Rom::new(&v).unwrap().mirroring, 0);
    }

    #[test]
    fn trainer_handled() {
        let prg = vec![0xFFu8; 0x4000];
        let mut data = fake_header(1, 0, 0x04, 0x00);
        data.extend(&[0xAA; 512]);
        data.extend(&prg);
        let rom = Rom::new(&data).unwrap();
        assert_eq!(rom.prg[0], 0xFF);
        assert!(rom.has_chr_ram);
    }

    #[test]
    fn read_prg_16kb_mirror() {
        let mut prg = vec![0u8; 0x4000];
        prg[0] = 0x11;
        prg[0x3FFF] = 0x22;
        let mut data = fake_header(1, 0, 0x00, 0x00);
        data.extend(&prg);
        let rom = Rom::new(&data).unwrap();

        assert_eq!(rom.read_prg(0x8000), 0x11);
        assert_eq!(rom.read_prg(0xC000), 0x11);
        assert_eq!(rom.read_prg(0xBFFF), 0x22);
        assert_eq!(rom.read_prg(0xFFFF), 0x22);
    }

    #[test]
    fn read_prg_32kb() {
        let mut prg = vec![0u8; 0x8000];
        prg[0] = 0x33;
        prg[0x7FFF] = 0x44;
        let mut data = fake_header(2, 0, 0x00, 0x00);
        data.extend(&prg);
        let rom = Rom::new(&data).unwrap();

        assert_eq!(rom.read_prg(0x8000), 0x33);
        assert_eq!(rom.read_prg(0xFFFF), 0x44);
    }

    #[test]
    fn chr_ram_zeros_when_no_chr() {
        let prg = vec![0u8; 0x4000];
        let mut data = fake_header(1, 0, 0x00, 0x00);
        data.extend(&prg);
        let rom = Rom::new(&data).unwrap();

        assert!(rom.has_chr_ram);
        assert_eq!(rom.chr[0], 0);
        assert_eq!(rom.chr[0x1FFF], 0);
    }

    #[test]
    #[cfg(feature = "std")]
    fn download_and_parse_nova() {
        let resp = ureq::get(
            "https://github.com/NovaSquirrel/NovaTheSquirrel/releases/download/v1.0.6a/nova.nes",
        )
        .call()
        .unwrap();
        let data = resp.into_body().read_to_vec().unwrap();
        assert!(data.len() > 16);
        let rom = Rom::new(&data).unwrap();
        assert!(rom.prg_size > 0);
        assert_eq!(rom.mapper, 1);
    }
}
