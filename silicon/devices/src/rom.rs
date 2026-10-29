use ld60_machine::Image;

use crate::ProgramError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RomKind {
    Mask,
    Eprom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rom {
    kind: RomKind,
    image: Image,
}

impl Rom {
    pub fn new(kind: RomKind, image: Image) -> Self {
        Self { kind, image }
    }

    pub fn kind(&self) -> RomKind {
        self.kind
    }

    pub fn image(&self) -> &Image {
        &self.image
    }

    pub fn program(&mut self, wanted: &Image) -> Result<(), ProgramError> {
        if self.kind == RomKind::Mask {
            return Err(ProgramError::MaskRom);
        }
        if wanted.len() != self.image.len() || wanted.word_bits() != self.image.word_bits() {
            return Err(ProgramError::SizeMismatch);
        }
        let conflict = self
            .image
            .words()
            .iter()
            .zip(wanted.words())
            .position(|(&current, &new)| !can_program(current, new));
        match conflict {
            Some(addr) => Err(ProgramError::BitConflict { addr }),
            None => {
                self.image = wanted.clone();
                Ok(())
            }
        }
    }

    pub fn erase(&mut self) -> Result<(), ProgramError> {
        if self.kind == RomKind::Mask {
            return Err(ProgramError::MaskRom);
        }
        self.image = Image::blank(self.image.len(), self.image.word_bits());
        Ok(())
    }

    /// Sets bits to 1, as UV light does over time.
    pub fn fade(&mut self, addr: usize, mask: u16) {
        self.image.set(addr, self.image.get(addr) | mask);
    }
}

/// EPROM programming can only clear bits. Setting a bit back to 1 needs an erase.
pub fn can_program(current: u16, wanted: u16) -> bool {
    current & wanted == wanted
}
