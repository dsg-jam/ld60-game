/// ROM contents. Words are up to 16 bits wide. Erased words have all bits set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    word_bits: u8,
    words: Vec<u16>,
}

impl Image {
    pub fn blank(len: usize, word_bits: u8) -> Self {
        assert!(
            (1..=16).contains(&word_bits),
            "word width must be 1 to 16 bits"
        );
        let mask = mask(word_bits);
        Self {
            word_bits,
            words: vec![mask; len],
        }
    }

    pub fn word_bits(&self) -> u8 {
        self.word_bits
    }

    pub fn mask(&self) -> u16 {
        mask(self.word_bits)
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub fn get(&self, addr: usize) -> u16 {
        self.words[addr]
    }

    pub fn set(&mut self, addr: usize, value: u16) {
        self.words[addr] = value & self.mask();
    }

    pub fn words(&self) -> &[u16] {
        &self.words
    }
}

fn mask(word_bits: u8) -> u16 {
    u16::MAX >> (16 - word_bits)
}
