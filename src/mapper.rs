#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mapper {
    pub id: u8,
    pub mirroring: u8,
}

impl Mapper {
    pub fn from_header(flags6: u8, flags7: u8) -> Self {
        Self {
            id: (flags7 & 0xF0) | (flags6 >> 4),
            mirroring: flags6 & 0x01,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mapper_and_mirroring_from_header() {
        let mapper = Mapper::from_header(0x01, 0x10);
        assert_eq!(mapper.id, 0x10);
        assert_eq!(mapper.mirroring, 0x01);
    }

    #[test]
    fn keeps_mirroring_bit_when_parsing_header() {
        let mapper = Mapper::from_header(0x01, 0x00);
        assert_eq!(mapper.id, 0);
        assert_eq!(mapper.mirroring, 1);
    }
}
