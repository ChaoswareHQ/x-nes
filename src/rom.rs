use alloc::vec::Vec;
use crate::mapper::Mapper;

pub struct Rom {
    pub prg: Vec<u8>,
    pub chr: Vec<u8>,
    pub mapper_id: u8,
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
        let mapper_id = (flags7 & 0xF0) | (flags6 >> 4);
        let mirroring = flags6 & 0x01;
        let has_chr_ram = chr_8kb == 0;

        let prg_size = prg_16kb * 0x4000;
        let header_size = if flags6 & 0x04 != 0 { 16 + 512 } else { 16 };

        let data_start = header_size;
        let data_end = data.len();
        let prg_end = (data_start + prg_size).min(data_end);

        let prg = data[data_start..prg_end].to_vec();
        let chr = if has_chr_ram {
            alloc::vec![0u8; 0x2000]
        } else {
            let chr_size = chr_8kb * 0x2000;
            let chr_start = data_start + prg_size;
            let chr_end = (chr_start + chr_size).min(data_end);
            data[chr_start..chr_end].to_vec()
        };

        Some(Self {
            prg,
            chr,
            mapper_id,
            mirroring,
            has_chr_ram,
        })
    }

    pub fn create_mapper(&self) -> Mapper {
        Mapper::from_ines(
            self.mapper_id,
            self.mirroring,
            &self.prg,
            &self.chr,
            self.has_chr_ram,
        )
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
        assert_eq!(rom.mapper_id, 0);
        assert_eq!(rom.mirroring, 0);
        assert!(rom.has_chr_ram);
        assert_eq!(rom.prg.len(), 0x4000);
    }

    #[test]
    fn nrom_16kb_prg_with_chr() {
        let prg = vec![0xCDu8; 0x4000];
        let chr = vec![0xEFu8; 0x2000];
        let mut data = fake_header(1, 1, 0x00, 0x00);
        data.extend(&prg);
        data.extend(&chr);
        let rom = Rom::new(&data).unwrap();
        assert_eq!(rom.mapper_id, 0);
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
        assert_eq!(rom.prg.len(), 0x8000);
        assert!(rom.has_chr_ram);
    }

    #[test]
    fn mapper_detected() {
        let data = fake_header(1, 0, 0x50, 0x10);
        let rom = Rom::new(&data).unwrap();
        assert_eq!(rom.mapper_id, 0x15);
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
    fn create_mapper_nrom() {
        let prg = vec![0xABu8; 0x4000];
        let mut data = fake_header(1, 0, 0x00, 0x00);
        data.extend(&prg);
        let rom = Rom::new(&data).unwrap();
        let mut mapper = rom.create_mapper();
        assert_eq!(mapper.cpu_read(0x8000), 0xAB);
        assert_eq!(mapper.cpu_read(0xC000), 0xAB);
    }

    #[test]
    fn create_mapper_mmc3() {
        let prg = vec![0xABu8; 0x8000];
        let mut data = fake_header(2, 0, 0x00, 0x00);
        data.extend(&prg);
        let rom = Rom::new(&data).unwrap();
        let mut mapper = rom.create_mapper();
        // $E000 should read from last 8KB bank
        assert_eq!(mapper.cpu_read(0xE000), 0xAB);
    }

    #[test]
    fn create_mapper_uxrom() {
        let prg = vec![0xAAu8; 0x8000];
        let mut data = fake_header(2, 0, 0x00, 0x00);
        data.extend(&prg);
        let rom = Rom::new(&data).unwrap();
        let mut mapper = rom.create_mapper();
        // UxROM: $8000 = bank 0 (switchable), $C000 = last bank (fixed)
        assert_eq!(mapper.cpu_read(0x8000), 0xAA);
        // Write to switch to bank 1
        mapper.cpu_write(0x8000, 1);
        assert_eq!(mapper.cpu_read(0x8000), 0xAA); // both banks same data
    }
}
